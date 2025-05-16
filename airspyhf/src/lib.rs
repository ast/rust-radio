use airspyhf_sys::*;

use thiserror::Error;

#[repr(i32)] // match C enum repr
#[derive(Error, Debug)]
pub enum AirspyHfError {
    #[error("AIRSPYHF_ERROR")]
    Generic = airspyhf_sys::airspyhf_error_AIRSPYHF_ERROR,
    #[error("AIRSPYHF_UNSUPPORTED")]
    Unsupported = airspyhf_sys::airspyhf_error_AIRSPYHF_UNSUPPORTED,
}

// Define the trait with the method
pub trait ToResult {
    fn to_result(self) -> Result<i32, AirspyHfError>;
}

// Implement the trait for i32
impl ToResult for i32 {
    fn to_result(self) -> Result<i32, AirspyHfError> {
        match self {
            res if res < 0 => match res {
                -1 => Err(AirspyHfError::Generic),
                -2 => Err(AirspyHfError::Unsupported),
                _ => panic!("Unknown error code"),
            },
            res => Ok(res),
        }
    }
}

fn lib_version() -> (u32, u32, u32) {
    let mut version = airspyhf_lib_version_t {
        major_version: 0,
        minor_version: 0,
        revision: 0,
    };
    unsafe {
        airspyhf_lib_version(&mut version);
    }
    (
        version.major_version,
        version.minor_version,
        version.revision,
    )
}

type SampleCallback = dyn FnMut(&[airspyhf_complex_float_t], u64) + Send + 'static;

struct CallbackContext {
    total_samples: usize,
    callback: Box<SampleCallback>,
}

struct Device {
    // Private device handle
    handle: *mut airspyhf_device_t,
    // Private context for transfer callback
    context: Option<*mut CallbackContext>,
}

extern "C" fn sample_block_callback(transfer: *mut airspyhf_transfer_t) -> i32 {
    // Safety: we trust libairspyhf to give us a valid pointer
    let transfer = unsafe { &mut *transfer };

    let context_ptr = transfer.ctx as *mut CallbackContext;
    let context = unsafe { &mut *context_ptr };

    let samples =
        unsafe { std::slice::from_raw_parts(transfer.samples, transfer.sample_count as usize) };

    // Store the number of samples in the context
    context.total_samples += samples.len();
    // Call the callback with the samples
    (context.callback)(samples, transfer.dropped_samples);

    0 // Success
}

impl Device {
    fn open() -> Result<Device, AirspyHfError> {
        let mut handle: *mut airspyhf_device_t = std::ptr::null_mut();
        unsafe { airspyhf_open(&mut handle) }.to_result()?;
        Ok(Device {
            handle,
            context: None,
        })
    }

    fn open_sn(serial: u64) -> Result<Device, AirspyHfError> {
        let mut handle: *mut airspyhf_device_t = std::ptr::null_mut();
        unsafe { airspyhf_open_sn(&mut handle, serial) }.to_result()?;
        Ok(Device {
            handle,
            context: None,
        })
    }

    fn close(mut self) -> Result<(), AirspyHfError> {
        let ret = unsafe { airspyhf_close(self.handle) };
        ret.to_result()?;
        self.handle = std::ptr::null_mut(); // Prevent double free
        Ok(())
    }

    fn output_size(&self) -> Result<i32, AirspyHfError> {
        let mut size: i32 = 0;
        let ret = unsafe { airspyhf_get_output_size(self.handle) };
        let size = ret.to_result()?;
        Ok(size)
    }
    pub fn start<F>(&mut self, callback: F) -> Result<(), AirspyHfError>
    where
        F: FnMut(&[airspyhf_complex_float_t], u64) + Send + 'static,
    {
        let context = Box::new(CallbackContext {
            total_samples: 0,
            callback: Box::new(callback),
        });

        let context_ptr = Box::into_raw(context);
        self.context = Some(context_ptr);

        let ret = unsafe {
            airspyhf_start(
                self.handle,
                Some(sample_block_callback),
                context_ptr as *mut std::ffi::c_void,
            )
        };
        ret.to_result()?;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), AirspyHfError> {
        let ret = unsafe { airspyhf_stop(self.handle) };

        // Safe to un-leak context
        if let Some(context_ptr) = self.context {
            unsafe {
                let _ = Box::from_raw(context_ptr);
            };
            self.context = None;
        }

        ret.to_result()?;
        Ok(())
    }

    fn is_streaming(&self) -> Result<bool, AirspyHfError> {
        let ret = unsafe { airspyhf_is_streaming(self.handle) };
        let is_streaming = ret.to_result()? == 1;
        Ok(is_streaming)
    }

    fn is_low_if(&self) -> Result<bool, AirspyHfError> {
        let ret = unsafe { airspyhf_is_low_if(self.handle) };
        let is_low_if = ret.to_result()? == 1;
        Ok(is_low_if)
    }

    fn get_samplerates(&self) -> Result<Vec<u32>, AirspyHfError> {
        let mut num_rates = 0;
        let ret = unsafe { airspyhf_get_samplerates(self.handle, &mut num_rates, 0) };
        ret.to_result()?;

        assert!(
            num_rates > 0,
            "Number of samplerates should be greater than 0",
        );

        // Create a Vec with the correct size to hold the rates
        let mut samplerates = vec![0; num_rates as usize];

        // Fill the Vec with the samplerates
        let ret =
            unsafe { airspyhf_get_samplerates(self.handle, samplerates.as_mut_ptr(), num_rates) };

        ret.to_result()?;

        Ok(samplerates)
    }
}

// Close on drop
impl Drop for Device {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                airspyhf_stop(self.handle);
                if let Some(ctx) = self.context.take() {
                    drop(Box::from_raw(ctx));
                }
                airspyhf_close(self.handle);
            }
        }
    }
}

fn list_devices() -> Result<Vec<u64>, AirspyHfError> {
    const MAX_DEVICES: usize = 10;

    let mut serials = vec![0; MAX_DEVICES];

    let ret = unsafe { airspyhf_list_devices(serials.as_mut_ptr(), serials.len() as i32) };

    let count = ret.to_result()?;

    serials.truncate(count as usize);
    Ok(serials)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[test]
    fn test_lib_version() {
        assert_eq!(mem::size_of::<airspyhf_transfer_t>(), 40);

        let (major, minor, revision) = lib_version();
        assert_eq!((major, minor, revision), (1, 8, 0));
    }

    #[test]
    fn test_list_devices() {
        let devices = list_devices();
        let num = devices.expect("Failed to list devices").len();
        // One device should be found
        assert_eq!(num, 1, "Expected one device, but found {}", num);
    }

    #[test]
    fn test_open() {
        let mut device = Device::open().expect("Failed to open device");
        assert!(!device.handle.is_null(), "Device handle is null");

        // Output size
        let output_size = device.output_size().expect("Failed to get output size");
        assert_eq!(
            output_size, 2048,
            "Expected output size to be 2048, but got {}",
            output_size
        );

        assert!(!device.is_streaming().expect("Failed to check streaming"));

        let samplerates = device.get_samplerates().expect("Failed to get samplerates");

        assert!(!samplerates.is_empty(), "Samplerates should not be empty");

        // Run device for a while
        device
            .start(|samples, dropped| {
                println!("Received {} samples, dropped {}", samples.len(), dropped);
            })
            .expect("Failed to start device");

        std::thread::sleep(std::time::Duration::from_secs(2));
        // Stop device
        device.stop().expect("Failed to stop device");

        device.close().expect("Failed to close device");
    }
}
