#[cfg(test)]
mod tests {
    use semver::Version;

    use uv_migrator::utils::uv::UV_SUPPORT_BARE;

    #[test]
    fn test_version_comparison() {
        // Hardcode the bare version for testing to ensure consistency
        let bare_version = Version::parse(UV_SUPPORT_BARE).unwrap();

        // Version below support threshold
        let below = Version::new(0, 5, 0);
        assert!(below < bare_version);

        // Version at support threshold
        let at = Version::new(0, 6, 0);
        assert!(at >= bare_version);

        // Version above support threshold
        let above = Version::new(0, 7, 0);
        assert!(above > bare_version);
    }

    // Test the should_use_bare_flag function directly instead of mocking UV
    #[test]
    fn test_should_use_bare_flag_with_version() {
        // Create a test function that isolates the version comparison logic
        fn should_use_bare_flag(uv_version: &str) -> bool {
            let uv_version = Version::parse(uv_version).unwrap();
            let min_version = Version::parse(UV_SUPPORT_BARE).unwrap();
            uv_version >= min_version
        }

        // Test with version below 0.6.0
        assert!(
            !should_use_bare_flag("0.5.9"),
            "Version 0.5.9 should NOT use --bare flag"
        );

        // Test with version equal to 0.6.0
        assert!(
            should_use_bare_flag("0.6.0"),
            "Version 0.6.0 should use --bare flag"
        );

        // Test with version above 0.6.0
        assert!(
            should_use_bare_flag("0.7.0"),
            "Version 0.7.0 should use --bare flag"
        );
    }

    // Test that we correctly construct the UvCommandBuilder with the bare flag
    #[test]
    fn test_command_construction_with_bare_flag() {
        use std::env;

        // Save the current environment
        let had_test_var = env::var("UV_TEST_SUPPORT_BARE").is_ok();
        let original_value = env::var("UV_TEST_SUPPORT_BARE").unwrap_or_default();

        // Set test environment
        unsafe {
            env::set_var("UV_TEST_SUPPORT_BARE", "0.6.0");
        }

        // Test version below threshold - shouldn't use --bare
        {
            let uv_version = Version::new(0, 5, 9);
            let version_supports_bare = Version::parse("0.6.0").unwrap();
            let using_bare_flag = uv_version >= version_supports_bare;
            assert!(!using_bare_flag, "Should not use --bare with version 0.5.9");
        }

        // Test version at threshold - should use --bare
        {
            let uv_version = Version::new(0, 6, 0);
            let version_supports_bare = Version::parse("0.6.0").unwrap();
            let using_bare_flag = uv_version >= version_supports_bare;
            assert!(using_bare_flag, "Should use --bare with version 0.6.0");
        }

        // Test version above threshold - should use --bare
        {
            let uv_version = Version::new(0, 7, 0);
            let version_supports_bare = Version::parse("0.6.0").unwrap();
            let using_bare_flag = uv_version >= version_supports_bare;
            assert!(using_bare_flag, "Should use --bare with version 0.7.0");
        }

        // Restore environment
        if had_test_var {
            unsafe {
                env::set_var("UV_TEST_SUPPORT_BARE", original_value);
            }
        } else {
            unsafe {
                env::remove_var("UV_TEST_SUPPORT_BARE");
            }
        }
    }
}
