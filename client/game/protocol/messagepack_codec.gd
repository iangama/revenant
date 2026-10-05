extends RefCounted

var _failed := false


func encode_map(value: Dictionary) -> PackedByteArray:
	_failed = false
	return _encode_map(value)


func encode_array(value: Array) -> PackedByteArray:
	_failed = false
	return _encode_array(value)


func has_failed() -> bool:
	return _failed


func decode_value(bytes: PackedByteArray, offset := 0) -> Array:
	if offset >= bytes.size():
		return []
	var marker := bytes[offset]
	if marker <= 0x7f:
		return [marker, offset + 1]
	if marker >= 0xe0:
		return [marker - 0x100, offset + 1]
	if marker >= 0xa0 and marker <= 0xbf:
		return _decode_string(bytes, offset + 1, marker & 0x1f)
	if marker >= 0x80 and marker <= 0x8f:
		return _decode_map(bytes, offset + 1, marker & 0x0f)
	if marker >= 0x90 and marker <= 0x9f:
		return _decode_array(bytes, offset + 1, marker & 0x0f)
	if marker == 0xc0:
		return [null, offset + 1]
	if marker == 0xc2:
		return [false, offset + 1]
	if marker == 0xc3:
		return [true, offset + 1]
	if marker == 0xcc and offset + 1 < bytes.size():
		return [bytes[offset + 1], offset + 2]
	if marker == 0xcd and offset + 2 < bytes.size():
		return [(bytes[offset + 1] << 8) | bytes[offset + 2], offset + 3]
	if marker == 0xce and offset + 4 < bytes.size():
		var value32 := 0
		for index in range(1, 5):
			value32 = (value32 << 8) | bytes[offset + index]
		return [value32, offset + 5]
	if marker == 0xcf and offset + 8 < bytes.size():
		var value64 := 0
		for index in range(1, 9):
			value64 = (value64 << 8) | bytes[offset + index]
		return [value64, offset + 9]
	if marker == 0xd0 and offset + 1 < bytes.size():
		return _decode_signed_integer(bytes, offset, 1)
	if marker == 0xd1 and offset + 2 < bytes.size():
		return _decode_signed_integer(bytes, offset, 2)
	if marker == 0xd2 and offset + 4 < bytes.size():
		return _decode_signed_integer(bytes, offset, 4)
	if marker == 0xd3 and offset + 8 < bytes.size():
		return _decode_signed_integer(bytes, offset, 8)
	if marker == 0xd9 and offset + 1 < bytes.size():
		return _decode_string(bytes, offset + 2, bytes[offset + 1])
	if marker == 0xdc and offset + 2 < bytes.size():
		return _decode_array(bytes, offset + 3, (bytes[offset + 1] << 8) | bytes[offset + 2])
	if marker == 0xde and offset + 2 < bytes.size():
		return _decode_map(bytes, offset + 3, (bytes[offset + 1] << 8) | bytes[offset + 2])
	return []


func _decode_map(bytes: PackedByteArray, offset: int, length: int) -> Array:
	var result := {}
	var cursor := offset
	for _entry in range(length):
		var key := decode_value(bytes, cursor)
		if key.is_empty() or not key[0] is String:
			return []
		cursor = key[1]
		var item := decode_value(bytes, cursor)
		if item.is_empty():
			return []
		cursor = item[1]
		result[key[0]] = item[0]
	return [result, cursor]


func _decode_array(bytes: PackedByteArray, offset: int, length: int) -> Array:
	var result := []
	var cursor := offset
	for _entry in range(length):
		var item := decode_value(bytes, cursor)
		if item.is_empty():
			return []
		cursor = item[1]
		result.append(item[0])
	return [result, cursor]


func _encode_map(value: Dictionary) -> PackedByteArray:
	var bytes := PackedByteArray()
	if value.size() <= 15:
		bytes.append(0x80 | value.size())
	elif value.size() <= 0xffff:
		bytes.append_array(PackedByteArray([0xde, (value.size() >> 8) & 0xff, value.size() & 0xff]))
	else:
		push_error("MessagePack map exceeds the supported 16-bit length")
		_failed = true
		return bytes
	for key in value:
		bytes.append_array(_encode_string(key))
		bytes.append_array(_encode_value(value[key]))
	return bytes


func _encode_array(value: Array) -> PackedByteArray:
	var bytes := PackedByteArray()
	if value.size() <= 15:
		bytes.append(0x90 | value.size())
	elif value.size() <= 0xffff:
		bytes.append_array(PackedByteArray([0xdc, (value.size() >> 8) & 0xff, value.size() & 0xff]))
	else:
		push_error("MessagePack array exceeds the supported 16-bit length")
		_failed = true
		return bytes
	for item in value:
		bytes.append_array(_encode_value(item))
	return bytes


func _encode_value(value: Variant) -> PackedByteArray:
	if value == null:
		return PackedByteArray([0xc0])
	if value is String:
		return _encode_string(value)
	if value is int:
		return _encode_integer(value)
	if value is bool:
		return PackedByteArray([0xc3 if value else 0xc2])
	if value is Array:
		return _encode_array(value)
	if value is Dictionary:
		return _encode_map(value)
	push_error("unsupported MessagePack value")
	_failed = true
	return PackedByteArray()


func _encode_integer(value: int) -> PackedByteArray:
	if value >= 0 and value <= 127:
		return PackedByteArray([value])
	if value >= -32 and value < 0:
		return PackedByteArray([256 + value])
	if value >= 0 and value <= 0xff:
		return PackedByteArray([0xcc, value])
	if value >= 0 and value <= 0xffff:
		return _encode_integer_bytes(0xcd, value, 2)
	if value >= 0 and value <= 0xffffffff:
		return _encode_integer_bytes(0xce, value, 4)
	if value >= 0:
		return _encode_integer_bytes(0xcf, value, 8)
	if value >= -0x80:
		return _encode_integer_bytes(0xd0, value, 1)
	if value >= -0x8000:
		return _encode_integer_bytes(0xd1, value, 2)
	if value >= -0x80000000:
		return _encode_integer_bytes(0xd2, value, 4)
	return _encode_integer_bytes(0xd3, value, 8)


func _encode_integer_bytes(marker: int, value: int, width: int) -> PackedByteArray:
	var bytes := PackedByteArray([marker])
	for shift in range((width - 1) * 8, -1, -8):
		bytes.append((value >> shift) & 0xff)
	return bytes


func _decode_signed_integer(bytes: PackedByteArray, offset: int, width: int) -> Array:
	var value := int(bytes[offset + 1])
	if value >= 0x80:
		value -= 0x100
	for index in range(2, width + 1):
		value = (value << 8) | bytes[offset + index]
	return [value, offset + width + 1]


func _encode_string(value: String) -> PackedByteArray:
	var utf8 := value.to_utf8_buffer()
	var bytes := PackedByteArray()
	if utf8.size() <= 31:
		bytes.append(0xa0 | utf8.size())
	else:
		bytes.append(0xd9)
		bytes.append(utf8.size())
	bytes.append_array(utf8)
	return bytes


func _decode_string(bytes: PackedByteArray, offset: int, length: int) -> Array:
	if offset + length > bytes.size():
		return []
	return [bytes.slice(offset, offset + length).get_string_from_utf8(), offset + length]
