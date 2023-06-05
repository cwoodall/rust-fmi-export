#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CString, CStr};
    use std::os::raw::{c_char};

    pub extern "cdecl" fn fmi2GetVersion() -> *const c_char {
        // Create a c-string from a rust string and report it
        let version = CString::new("2.0").unwrap();
        version.into_raw() 
    }

    #[test]
    fn test_type_imports() {
        let a : fmi2Boolean = 1;
        assert_eq!(a, 1);
    }

    #[test]
    fn test_get_version_override() {
        // Run the implemented fmi2GetVersion() function
        let c_buf: *const c_char = fmi2GetVersion();

        // convert the outputted *const c_char to a string
        let c_str: &CStr = unsafe { CStr::from_ptr(c_buf) };
        let str_slice: &str = c_str.to_str().unwrap();
        
        assert_eq!("2.0", str_slice);
    }

}
