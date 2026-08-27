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
	if marker >= 0xa0 and marker <= 0xbf:
		return _decode_string(bytes, offset + 1, marker & 0x1f)
	if marker >= 0x80 and marker <= 0x8f:
		var result := {}
		var cursor := offset + 1
		for _entry in range(marker & 0x0f):
			var key := decode_value(bytes, cursor)
			if key.is_empty():
				return []
			cursor = key[1]
			var item := decode_value(bytes, cursor)
			if item.is_empty():
				return []
			cursor = item[1]
			result[key[0]] = item[0]
		return [result, cursor]
	if marker >= 0x90 and marker <= 0x9f:
		var result := []
		var cursor := offset + 1
		for _entry in range(marker & 0x0f):
			var item := decode_value(bytes, cursor)
			if item.is_empty():
				return []
			cursor = item[1]
			result.append(item[0])
		return [result, cursor]
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
	if marker == 0xd9 and offset + 1 < bytes.size():
		return _decode_string(bytes, offset + 2, bytes[offset + 1])
	return []


func _encode_map(value: Dictionary) -> PackedByteArray:
	var bytes := PackedByteArray([0x80 | value.size()])
	for key in value:
		bytes.append_array(_encode_string(key))
		var item = value[key]
		if item is String:
			bytes.append_array(_encode_string(item))
		elif item is int:
			bytes.append_array(_encode_integer(item))
		elif item is Array:
			bytes.append_array(_encode_array(item))
		else:
			push_error("unsupported M1 MessagePack value")
			_failed = true
	return bytes


func _encode_array(value: Array) -> PackedByteArray:
	var bytes := PackedByteArray([0x90 | value.size()])
	for item in value:
		if item is int:
			bytes.append_array(_encode_integer(item))
		else:
			push_error("unsupported M8 MessagePack array value")
			_failed = true
	return bytes


func _encode_integer(value: int) -> PackedByteArray:
	if value >= 0 and value <= 127:
		return PackedByteArray([value])
	if value >= -32 and value < 0:
		return PackedByteArray([256 + value])
	push_error("unsupported MessagePack integer value: %d" % value)
	_failed = true
	return PackedByteArray()


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
