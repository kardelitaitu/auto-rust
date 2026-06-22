"""Clean up the corrupted test_css_escape_all_special_chars_mixed function."""
with open('src/utils/twitter/twitteractivity_dive.rs', 'rb') as f:
    data = bytearray(f.read())

# Find the corrupted section - three #[test] lines followed by the broken function
marker = b'\n    #[test]\n    #[test]\n    #[test]\n    fn test_css_escape_all_special_chars_mixed'
idx = data.find(marker)
if idx < 0:
    # Try finding just the function name
    idx = data.find(b'test_css_escape_all_special_chars_mixed')
    if idx >= 0:
        # Find the preceding #[test]
        test_idx = data.rfind(b'#[test]', 0, idx)
        if test_idx >= 0:
            idx = test_idx

if idx >= 0:
    # Find the end of this function - look for the pattern where the test framework
    # continues after the broken function. Search for the next clean #[test]
    next_test = data.find(b'\n    #[test]\n    fn test_css_escape_only_special_chars', idx)
    if next_test < 0:
        next_test = data.find(b'\n    #[test]\n    fn test_css_escape_empty_string', idx)
    if next_test < 0:
        next_test = data.find(b'    #[test]\n    fn test_css_escape_only_special_chars', idx)
    
    if next_test >= 0:
        # Replace everything from the corrupted test to the next clean test
        new_test = b'''
    #[test]
    fn test_css_escape_all_special_chars_mixed() {
        // Input: a'b\\\\"c (a, ', b, 2 backslashes, ", c)
        // Build input at runtime to avoid V in Rust string literal
        let input = format!(r"a'b\\\\{}c", '"');
        let result = css_escape_attr_value(&input);
        // Verify no unescaped special chars remain
        for (i, ch) in result.chars().enumerate() {
            if ch == '\\'' || ch == '\"' {
                assert!(i > 0 && result.as_bytes()[i - 1] == b'\\\\',
                    "special char at position {i} not escaped");
            }
        }
    }

'''
        data[idx:next_test] = new_test
        with open('src/utils/twitter/twitteractivity_dive.rs', 'wb') as f:
            f.write(data)
        print(f"SUCCESS: Replaced bytes {idx}-{next_test}")
    else:
        print("FAILED: Could not find next test")
else:
    print("FAILED: Could not find corrupted section")
    # Show what we found around 'all_special_chars'
    ctx = data.find(b'all_special_chars')
    if ctx >= 0:
        print(repr(data[ctx-100:ctx+400]))
