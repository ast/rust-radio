//! Safe Rust wrapper around `libairspyhf`.
//!
//! Provides [`Device`] for opening, configuring, and streaming IQ samples
//! ([`Complex32`]) from AirspyHF+ receivers. IQ samples are normalized to the
//! range `[-1.0, 1.0]` and delivered in fixed-size blocks (see
//! [`Device::output_size`]).
//!
//! # Example
//!
//! ```no_run
//! use airspyhf::Device;
//! use std::ops::ControlFlow;
//!
//! let mut device = Device::open()?;
//! device.set_frequency(7.2e6)?;
//! device.start(|samples, dropped| {
//!     println!("got {} samples ({} dropped)", samples.len(), dropped);
//!     ControlFlow::Continue(())
//! })?;
//! std::thread::sleep(std::time::Duration::from_secs(2));
//! device.stop()?;
//! # Ok::<(), airspyhf::AirspyHfError>(())
//! ```

use airspyhf_sys::*;

use num_complex::Complex32;
use std::ops::ControlFlow;
use std::ptr::NonNull;
use thiserror::Error;

/// Errors returned by `libairspyhf` calls and this wrapper.
#[derive(Error, Debug)]
pub enum AirspyHfError {
    /// `AIRSPYHF_ERROR` (-1) — generic failure from the C library.
    #[error("AIRSPYHF_ERROR")]
    Generic,
    /// `AIRSPYHF_UNSUPPORTED` (-2) — operation not supported by this device or firmware.
    #[error("AIRSPYHF_UNSUPPORTED")]
    Unsupported,
    /// A negative return code that this wrapper does not recognize.
    #[error("unknown airspyhf error code: {0}")]
    Unknown(i32),
    /// [`Device::start`] was called while the device is already streaming.
    #[error("device is already streaming")]
    AlreadyStreaming,
}

/// Convert a `libairspyhf` C return value (negative = error, non-negative = ok)
/// into a `Result`.
pub trait ToResult {
    fn to_result(self) -> Result<i32, AirspyHfError>;
}

impl ToResult for i32 {
    fn to_result(self) -> Result<i32, AirspyHfError> {
        match self {
            res if res >= 0 => Ok(res),
            -1 => Err(AirspyHfError::Generic),
            -2 => Err(AirspyHfError::Unsupported),
            other => Err(AirspyHfError::Unknown(other)),
        }
    }
}

/// Returns the runtime version of `libairspyhf` as `(major, minor, revision)`.
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

/// Streaming callback signature.
///
/// Invoked from libairspyhf's USB worker thread for each block of IQ samples.
/// Because it runs off-thread the closure must be `Send + 'static` and cannot
/// borrow anything from the calling stack.
///
/// Arguments:
/// - `samples` — a block of normalized IQ samples (length is fixed per device,
///   see [`Device::output_size`]).
/// - `dropped` — number of samples dropped by the USB driver since the last
///   callback (0 in steady state).
///
/// Return:
/// - [`ControlFlow::Continue`] to keep streaming.
/// - [`ControlFlow::Break`] to ask libairspyhf to stop.
pub type SampleCallback =
    dyn FnMut(&[Complex32], u64) -> ControlFlow<()> + Send + 'static;

/// Heap-allocated callback closure handed to the C library by raw pointer.
/// See [`Device::start`] for the lifecycle and ownership rules.
struct CallbackContext {
    callback: Box<SampleCallback>,
}

/// An open AirspyHF+ receiver.
///
/// Drop closes the device and releases any in-flight streaming callback. There
/// is no need to call [`Device::stop`] explicitly before drop.
pub struct Device {
    handle: NonNull<airspyhf_device_t>,
    /// Owning pointer to the streaming callback context. Logically a
    /// `Box<CallbackContext>` that has been leaked across the FFI boundary so
    /// libairspyhf can hand the same `*mut c_void` back to us in
    /// [`sample_block_callback`]. `Some` while streaming, `None` otherwise.
    /// See [`Device::start`] / [`Device::stop`] for the full lifecycle.
    context: Option<*mut CallbackContext>,
}

// SAFETY: libairspyhf owns the device handle and synchronizes access from its
// internal USB worker threads; documented API calls (set_frequency, etc.) are
// safe while streaming. The `CallbackContext` pointer is an owning leaked box
// that is only dereferenced on the USB worker thread (via the C trampoline)
// or after `airspyhf_stop` has joined that thread, never concurrently from
// Rust. Neither pointer exposes interior aliasing to Rust callers.
unsafe impl Send for Device {}
unsafe impl Sync for Device {}

/// Trampoline registered with libairspyhf. Forwards each transfer to the
/// user's `FnMut` stored in the [`CallbackContext`] referenced by `transfer.ctx`.
extern "C" fn sample_block_callback(transfer: *mut airspyhf_transfer_t) -> i32 {
    // SAFETY: libairspyhf guarantees a valid, non-null transfer pointer for each callback.
    let transfer = unsafe { &mut *transfer };

    let context_ptr = transfer.ctx as *mut CallbackContext;
    let context = unsafe { &mut *context_ptr };

    // SAFETY: airspyhf_complex_float_t and Complex<f32> are both #[repr(C)] with identical layout.
    let samples: &[Complex32] = unsafe {
        std::slice::from_raw_parts(
            transfer.samples as *const Complex32,
            transfer.sample_count as usize,
        )
    };

    match (context.callback)(samples, transfer.dropped_samples) {
        ControlFlow::Continue(()) => 0,
        ControlFlow::Break(()) => 1,
    }
}

impl Device {
    /// Open the first available AirspyHF+ device.
    pub fn open() -> Result<Device, AirspyHfError> {
        let mut handle: *mut airspyhf_device_t = std::ptr::null_mut();
        unsafe { airspyhf_open(&mut handle) }.to_result()?;
        Ok(Device {
            handle: NonNull::new(handle).expect("device nullptr even if open() succeeded"),
            context: None,
        })
    }

    /// Open a specific device by serial number (as returned by [`list_devices`]).
    pub fn open_sn(serial: u64) -> Result<Device, AirspyHfError> {
        let mut handle: *mut airspyhf_device_t = std::ptr::null_mut();
        unsafe { airspyhf_open_sn(&mut handle, serial) }.to_result()?;
        Ok(Device {
            handle: NonNull::new(handle).expect("Failed to create NonNull"),
            context: None,
        })
    }

    /// Number of IQ samples libairspyhf delivers per streaming callback.
    pub fn output_size(&self) -> Result<usize, AirspyHfError> {
        let ret = unsafe { airspyhf_get_output_size(self.handle.as_ptr()) };
        Ok(ret.to_result()? as usize)
    }

    /// Begin streaming IQ samples. `callback` is invoked from libairspyhf's
    /// USB worker thread for each block until [`stop`](Self::stop) is called,
    /// the device is dropped, or the closure returns [`ControlFlow::Break`].
    ///
    /// Returns [`AirspyHfError::AlreadyStreaming`] if the device is already
    /// streaming — call [`stop`](Self::stop) first to swap callbacks.
    ///
    /// # Callback context lifecycle
    ///
    /// The closure is boxed into a `CallbackContext` and then **leaked** with
    /// [`Box::into_raw`] to obtain a stable `*mut c_void` that we hand to
    /// `airspyhf_start`. libairspyhf passes that same pointer back to our
    /// internal trampoline on every transfer, so the box must remain valid
    /// for the entire streaming session — it is owned by neither the C
    /// library nor the calling Rust stack.
    ///
    /// Reclamation happens in exactly one place per session:
    ///
    /// - [`stop`](Self::stop) calls `airspyhf_stop`, which joins the USB
    ///   worker threads (`kill_io_threads` → `pthread_join`). Once it returns,
    ///   no callback can fire, so the box is reconstituted with
    ///   `Box::from_raw` and dropped.
    /// - [`Drop`] does the same on the way out, in case the user never called
    ///   `stop`.
    ///
    /// If `airspyhf_start` itself fails, no worker thread was spawned, so we
    /// reclaim the box immediately and do not store the pointer in
    /// `self.context`.
    pub fn start<F>(&mut self, callback: F) -> Result<(), AirspyHfError>
    where
        F: FnMut(&[Complex32], u64) -> ControlFlow<()> + Send + 'static,
    {
        if self.context.is_some() {
            return Err(AirspyHfError::AlreadyStreaming);
        }

        // Box the closure so it has a stable address, then leak the box across
        // the FFI boundary. The matching `Box::from_raw` lives in `stop`/`Drop`.
        let context = Box::new(CallbackContext {
            callback: Box::new(callback),
        });
        let context_ptr = Box::into_raw(context);

        let ret = unsafe {
            airspyhf_start(
                self.handle.as_ptr(),
                Some(sample_block_callback),
                context_ptr as *mut std::ffi::c_void,
            )
        };

        if let Err(e) = ret.to_result() {
            // SAFETY: airspyhf_start failed, so no callback thread was spawned
            // and nothing else holds a reference to the context. Reclaim the
            // leaked box immediately to avoid a permanent leak.
            unsafe { drop(Box::from_raw(context_ptr)) };
            return Err(e);
        }

        // Streaming is live; remember the pointer so stop()/Drop can free it.
        self.context = Some(context_ptr);
        Ok(())
    }

    /// Stop streaming and release the callback context.
    ///
    /// Safe to call when not streaming. Calling [`start`](Self::start) again
    /// afterwards is allowed.
    pub fn stop(&mut self) -> Result<(), AirspyHfError> {
        let ret = unsafe { airspyhf_stop(self.handle.as_ptr()) };

        // SAFETY: airspyhf_stop joins the USB worker threads (pthread_join in
        // kill_io_threads in libairspyhf), so no callback can be running or
        // start after this returns. That makes it safe to reclaim the box that
        // start() leaked across the FFI boundary.
        if let Some(context_ptr) = self.context.take() {
            unsafe { drop(Box::from_raw(context_ptr)) };
        }

        ret.to_result()?;
        Ok(())
    }

    /// Whether the device is currently streaming IQ samples.
    pub fn is_streaming(&self) -> Result<bool, AirspyHfError> {
        let ret = unsafe { airspyhf_is_streaming(self.handle.as_ptr()) };
        let is_streaming = ret.to_result()? == 1;
        Ok(is_streaming)
    }

    /// Whether the device is operating in low-IF (rather than zero-IF) mode.
    pub fn is_low_if(&self) -> Result<bool, AirspyHfError> {
        let ret = unsafe { airspyhf_is_low_if(self.handle.as_ptr()) };
        let is_low_if = ret.to_result()? == 1;
        Ok(is_low_if)
    }

    /// Tune to `freq_hz` (Hz). Can be called while streaming.
    pub fn set_frequency(&self, freq_hz: f64) -> Result<(), AirspyHfError> {
        let ret = unsafe { airspyhf_set_freq_double(self.handle.as_ptr(), freq_hz) };
        ret.to_result()?;
        Ok(())
    }

    /// List the sample rates this device supports, in Hz.
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

    /// Set the active sample rate. Must be one of the values returned by
    /// [`get_samplerates`](Self::get_samplerates).
    pub fn set_samplerate(&self, samplerate: u32) -> Result<(), AirspyHfError> {
        let ret = unsafe { airspyhf_set_samplerate(self.handle.as_ptr(), samplerate) };
        ret.to_result()?;
        Ok(())
    }

    /// Read the firmware version string from the device (e.g. `"R3.0.7-BB"`).
    pub fn version_string(&self) -> Result<String, AirspyHfError> {
        const MAX_VERSION_LENGTH: usize = 64;

        let mut version = vec![0u8; MAX_VERSION_LENGTH];

        let ret = unsafe {
            airspyhf_version_string_read(
                self.handle.as_ptr(),
                version.as_mut_ptr() as *mut i8,
                version.len() as u8,
            )
        };
        ret.to_result()?;

        let end = version.iter().position(|&b| b == 0).unwrap_or(version.len());
        Ok(String::from_utf8_lossy(&version[..end]).into_owned())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: airspyhf_stop joins the USB worker threads, so once it
            // returns no callback can fire and the leaked context box from
            // start() is safe to reclaim. Mirrors stop(), but ignores the
            // return value because Drop cannot propagate errors.
            airspyhf_stop(self.handle.as_ptr());
            if let Some(ctx) = self.context.take() {
                drop(Box::from_raw(ctx));
            }
            airspyhf_close(self.handle.as_ptr());
        }
    }
}

/// Enumerate the serial numbers of all connected AirspyHF+ devices.
///
/// Pass any of these values to [`Device::open_sn`] to open a specific device.
pub fn list_devices() -> Result<Vec<u64>, AirspyHfError> {
    // First call with a null buffer to discover the device count.
    let count = unsafe { airspyhf_list_devices(std::ptr::null_mut(), 0) }.to_result()?;
    if count == 0 {
        return Ok(Vec::new());
    }

    let mut serials = vec![0u64; count as usize];
    let written =
        unsafe { airspyhf_list_devices(serials.as_mut_ptr(), count) }.to_result()?;
    serials.truncate(written as usize);
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
        assert_eq!((major, minor, revision), (1, 8, 1));
    }

    #[test]
    #[ignore]
    fn test_list_devices() {
        let devices = list_devices();
        let num = devices.expect("Failed to list devices").len();
        // One device should be found
        assert_eq!(num, 1, "Expected one device, but found {}", num);
    }

    #[test]
    #[ignore]
    fn test_open() {
        let mut device = Device::open().expect("Failed to open device");

        // Output size
        let output_size = device.output_size().expect("Failed to get output size");
        assert_eq!(
            output_size, 4096,
            "Expected output size to be 4096, but got {}",
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
                assert!(samples.len() == 4096, "No samples received");
                assert!(dropped == 0, "Dropped samples should be 0");
                ControlFlow::Continue(())
            })
            .expect("Failed to start device");

        std::thread::sleep(std::time::Duration::from_secs(2));
        // Device goes out of scope and is dropped here
    }
}
