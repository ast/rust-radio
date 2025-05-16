#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_airspyhf_transfer_t() {
        assert_eq!(mem::size_of::<airspyhf_transfer_t>(), 40);
    }

    #[test]
    fn test_airspyhf_complex_float_t() {
        // Check the size of the airspyhf_sample_type struct
        assert_eq!(mem::size_of::<airspyhf_complex_float_t>(), 8);
    }

    #[test]
    fn test_airspyhf_lib_version() {
        // Check the size of the airspyhf_lib_version struct
        assert_eq!(mem::size_of::<airspyhf_lib_version_t>(), 12);

        // Get lib version
        let mut version = airspyhf_lib_version_t {
            major_version: 0,
            minor_version: 0,
            revision: 0,
        };
        unsafe {
            airspyhf_lib_version(&mut version);
        }

        // Check the version
        assert_eq!(version.major_version, 1);
        assert_eq!(version.minor_version, 8);
        assert_eq!(version.revision, 0);
    }
}
