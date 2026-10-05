extends Translation

# Existing views assemble labels from server values. Translate their display
# templates without changing the canonical text stored by those views.
const CATALOG := preload("res://presentation/localization/pt_BR.po")
const CACHE_LIMIT := 512
var _patterns: Array[Dictionary] = []
var _cache := {}


func _init() -> void:
	locale = "pt_BR"
	var placeholder := RegEx.create_from_string("%(?:[-+0-9.]*[dfs]|%)")
	var messages := Array(CATALOG.get_message_list())
	messages.sort_custom(func(a: String, b: String) -> bool: return a.length() > b.length())
	for message: String in messages:
		var tokens := placeholder.search_all(message)
		if tokens.is_empty() or not RegEx.create_from_string("[A-Za-z]{2}").search(placeholder.sub(message, "", true)):
			continue
		var pattern := "(?s)^"
		var offset := 0
		var kinds: Array[String] = []
		for token in tokens:
			pattern += _escape(message.substr(offset, token.get_start() - offset))
			var kind := token.get_string().right(1)
			if kind == "%":
				pattern += "%"
			else:
				kinds.append(kind)
				pattern += "([-+]?[0-9]+)" if kind == "d" else ("([-+]?[0-9.]+)" if kind == "f" else "(.*?)")
			offset = token.get_end()
		pattern += _escape(message.substr(offset)) + "$"
		_patterns.append({"regex": RegEx.create_from_string(pattern), "kinds": kinds, "translation": str(CATALOG.get_message(message))})


func _get_message(source: StringName, _context: StringName) -> StringName:
	var text := str(source)
	if text.length() > 16384:
		return &""
	if _cache.has(text):
		return _cache[text]
	var translated := _resolve(text, 0)
	if _cache.size() >= CACHE_LIMIT:
		_cache.clear()
	_cache[text] = StringName(translated) if translated != text else &""
	return _cache[text]


func _resolve(text: String, depth: int) -> String:
	var exact := str(CATALOG.get_message(text))
	if not exact.is_empty():
		return exact
	if depth >= 4:
		return text
	for pattern in _patterns:
		var match_result: RegExMatch = pattern.regex.search(text)
		if match_result == null:
			continue
		var values: Array = []
		for index in pattern.kinds.size():
			var value := match_result.get_string(index + 1)
			match pattern.kinds[index]:
				"d": values.append(int(value))
				"f": values.append(float(value))
				_: values.append(_resolve(value, depth + 1))
		return pattern.translation % values
	for separator in ["\n", "  •  ", " • ", ", ", " → "]:
		if separator in text:
			var pieces: PackedStringArray = text.split(separator)
			for index in pieces.size():
				pieces[index] = _resolve(pieces[index], depth + 1)
			return separator.join(pieces)
	var quantity := RegEx.create_from_string("^(.*?)(  x[0-9]+)$").search(text)
	if quantity != null:
		return _resolve(quantity.get_string(1), depth + 1) + quantity.get_string(2)
	var numbered := RegEx.create_from_string("^([0-9]+  )(.*)$").search(text)
	if numbered != null:
		return numbered.get_string(1) + _resolve(numbered.get_string(2), depth + 1)
	return text


func _escape(text: String) -> String:
	var escaped := ""
	for character in text:
		if character in "\\.^$|?*+()[]{}":
			escaped += "\\"
		escaped += character
	return escaped
