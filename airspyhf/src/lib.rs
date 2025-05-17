use airspyhf_sys::*;

use num_complex::Complex;
use std::ptr::NonNull;
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

pub fn lib_version() -> (u32, u32, u32) {
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

type SampleCallback = dyn FnMut(&[Complex<f32>], u64) -> i32 + Send + 'static;

struct CallbackContext {
    total_samples: usize,
    callback: Box<SampleCallback>,
}

pub struct Device {
    // Private device handle
    handle: NonNull<airspyhf_device_t>, // *mut airspyhf_device_t,
    // Private context for transfer callback
    context: Option<*mut CallbackContext>,
}

extern "C" fn sample_block_callback(transfer: *mut airspyhf_transfer_t) -> i32 {
    // Safety: we trust libairspyhf to give us a valid pointer
    let transfer = unsafe { &mut *transfer };

    let context_ptr = transfer.ctx as *mut CallbackContext;
    let context = unsafe { &mut *context_ptr };

    // SAFETY: airspyhf_complex_float_t and Complex<f32> are both #[repr(C)] with identical layout
    let samples: &[Complex<f32>] = unsafe {
        std::slice::from_raw_parts(
            transfer.samples as *const Complex<f32>,
            transfer.sample_count as usize,
        )
    };

    // Store the number of samples in the context
    context.total_samples += samples.len();

    // Call the callback with the samples
    (context.callback)(samples, transfer.dropped_samples)
}

impl Device {
    pub fn open() -> Result<Device, AirspyHfError> {
        let mut handle: *mut airspyhf_device_t = std::ptr::null_mut();
        unsafe { airspyhf_open(&mut handle) }.to_result()?;
        Ok(Device {
            handle: NonNull::new(handle).expect("device nullptr even if open() succeeded"),
            context: None,
        })
    }

    pub fn open_sn(serial: u64) -> Result<Device, AirspyHfError> {
        let mut handle: *mut airspyhf_device_t = std::ptr::null_mut();
        unsafe { airspyhf_open_sn(&mut handle, serial) }.to_result()?;
        Ok(Device {
            handle: NonNull::new(handle).expect("Failed to create NonNull"),
            context: None,
        })
    }

    pub fn output_size(&self) -> Result<i32, AirspyHfError> {
        let mut size: i32 = 0;
        let ret = unsafe { airspyhf_get_output_size(self.handle.as_ptr()) };
        let size = ret.to_result()?;
        Ok(size)
    }
    pub fn start<F>(&mut self, callback: F) -> Result<(), AirspyHfError>
    where
        F: FnMut(&[Complex<f32>], u64) -> i32 + Send + 'static,
    {
        let context = Box::new(CallbackContext {
            total_samples: 0,
            callback: Box::new(callback),
        });

        let context_ptr = Box::into_raw(context);
        self.context = Some(context_ptr);

        let ret = unsafe {
            airspyhf_start(
                self.handle.as_ptr(),
                Some(sample_block_callback),
                context_ptr as *mut std::ffi::c_void,
            )
        };
        ret.to_result()?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), AirspyHfError> {
        let ret = unsafe { airspyhf_stop(self.handle.as_ptr()) };

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

    pub fn is_streaming(&self) -> Result<bool, AirspyHfError> {
        let ret = unsafe { airspyhf_is_streaming(self.handle.as_ptr()) };
        let is_streaming = ret.to_result()? == 1;
        Ok(is_streaming)
    }

    pub fn is_low_if(&self) -> Result<bool, AirspyHfError> {
        let ret = unsafe { airspyhf_is_low_if(self.handle.as_ptr()) };
        let is_low_if = ret.to_result()? == 1;
        Ok(is_low_if)
    }

    pub fn set_frequency(&self, freq_hz: f64) -> Result<(), AirspyHfError> {
        let ret = unsafe { airspyhf_set_freq_double(self.handle.as_ptr(), freq_hz) };
        ret.to_result()?;
        Ok(())
    }

    pub fn get_samplerates(&self) -> Result<Vec<u32>, AirspyHfError> {
        let mut num_rates = 0;
        let ret = unsafe { airspyhf_get_samplerates(self.handle.as_ptr(), &mut num_rates, 0) };
        ret.to_result()?;

        assert!(
            num_rates > 0,
            "Number of samplerates should be greater than 0",
        );

        // Create a Vec with the correct size to hold the rates
        let mut samplerates = vec![0; num_rates as usize];

        // Fill the Vec with the samplerates
        let ret = unsafe {
            airspyhf_get_samplerates(self.handle.as_ptr(), samplerates.as_mut_ptr(), num_rates)
        };

        ret.to_result()?;

        Ok(samplerates)
    }

    pub fn set_samplerate(&self, samplerate: u32) -> Result<(), AirspyHfError> {
        let ret = unsafe { airspyhf_set_samplerate(self.handle.as_ptr(), samplerate) };
        ret.to_result()?;
        Ok(())
    }

    pub fn version_string(&self) -> Result<String, AirspyHfError> {
        const MAX_VERSION_LENGTH: usize = 64;

        let mut version = vec![0; MAX_VERSION_LENGTH];

        let ret = unsafe {
            airspyhf_version_string_read(
                self.handle.as_ptr(),
                version.as_mut_ptr() as *mut i8,
                version.len() as u8,
            )
        };
        ret.to_result()?;

        // Convert to String
        let version_str = String::from_utf8_lossy(&version).to_string();
        Ok(version_str)
    }
}

// Close on drop
impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            airspyhf_stop(self.handle.as_ptr());
            // The callback won't be called anymore, so we can
            // safely drop the context
            if let Some(ctx) = self.context.take() {
                drop(Box::from_raw(ctx));
            }
            airspyhf_close(self.handle.as_ptr());
        }
    }
}

pub fn list_devices() -> Result<Vec<u64>, AirspyHfError> {
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

        // Set frequency
        let freq_hz = 7.2e6;
        device
            .set_frequency(freq_hz)
            .expect("Failed to set frequency");

        // Run device for a while
        device
            .start(|samples, dropped| {
                // println!("Received {} samples, dropped {}", samples.len(), dropped);
                // 0 = continue
                0
            })
            .expect("Failed to start device");

        std::thread::sleep(std::time::Duration::from_secs(2));
        // Device goes out of scope and is dropped here
    }
}
