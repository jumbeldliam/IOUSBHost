#![feature(type_alias_impl_trait)]
use core::ffi::c_void;
use core::future::Future;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr;
use core::ptr::NonNull;
use core::task::{Context, Poll, Waker};
use iousbhost_sys::*;

#[derive(Debug)]
pub enum UsbError {
    InvalidAddress = 1,
    ProtectionFailure = 2,
    NoSpace = 3,
    InvalidArgument = 4,
    Failure = 5,
    ResourceShortage = 6,
    NotReceiver = 7,
    NoAccess = 8,
    MemoryFailure = 9,
    MemoryError = 10,
    AlreadyInSet = 11,
    NotInSet = 12,
    NameExists = 13,
    Aborted = 14,
    InvalidName = 15,
    InvalidTask = 16,
    InvalidRight = 17,
    InvalidValue = 18,
    UrefsOverflow = 19,
    InvalidCapability = 20,
    RightExists = 21,
    InvalidHost = 22,
    MemoryPresent = 23,
    MemoryDataMoved = 24,
    MemoryRestartCopy = 25,
    InvalidProcessorSet = 26,
    PolicyLimit = 27,
    InvalidPolicy = 28,
    InvalidObject = 29,
    AlreadyWaiting = 30,
    DefaultSet = 31,
    ExceptionProtected = 32,
    InvalidLedger = 33,
    InvalidMemoryControl = 34,
    InvalidSecurity = 35,
    NotDepressed = 36,
    Terminated = 37,
    LockSetDestroyed = 38,
    LockUnstable = 39,
    LockOwned = 40,
    LockOwnedSelf = 41,
    SemaphoreDestroyed = 42,
    RpcServerTerminated = 43,
    RpcTerminateOrphan = 44,
    RpcContinueOrphan = 45,
    NotSupported = 46,
    NodeDown = 47,
    NotWaiting = 48,
    OperationTimedOut = 49,
    Unknown,
}

impl From<UsbError> for kern_return_t {
    fn from(_err: UsbError) -> i32 {
        todo!()
    }
}

impl From<kern_return_t> for UsbError {
    fn from(err: kern_return_t) -> UsbError {
        use UsbError as E;
        match err as u32 {
            KERN_INVALID_ADDRESS => E::InvalidAddress,
            KERN_PROTECTION_FAILURE => E::ProtectionFailure,
            KERN_NO_SPACE => E::NoSpace,
            KERN_INVALID_ARGUMENT => E::InvalidArgument,
            KERN_FAILURE => E::Failure,
            KERN_RESOURCE_SHORTAGE => E::ResourceShortage,
            KERN_NOT_RECEIVER => E::NotReceiver,
            KERN_NO_ACCESS => E::NoAccess,
            KERN_MEMORY_FAILURE => E::MemoryFailure,
            KERN_MEMORY_ERROR => E::MemoryError,
            KERN_ALREADY_IN_SET => E::AlreadyInSet,
            KERN_NOT_IN_SET => E::NotInSet,
            KERN_NAME_EXISTS => E::NameExists,
            KERN_ABORTED => E::Aborted,
            KERN_INVALID_NAME => E::InvalidName,
            KERN_INVALID_TASK => E::InvalidTask,
            KERN_INVALID_RIGHT => E::InvalidRight,
            KERN_INVALID_VALUE => E::InvalidValue,
            KERN_UREFS_OVERFLOW => E::UrefsOverflow,
            KERN_INVALID_CAPABILITY => E::InvalidCapability,
            KERN_RIGHT_EXISTS => E::RightExists,
            KERN_INVALID_HOST => E::InvalidHost,
            KERN_MEMORY_PRESENT => E::MemoryPresent,
            KERN_MEMORY_DATA_MOVED => E::MemoryDataMoved,
            KERN_MEMORY_RESTART_COPY => E::MemoryRestartCopy,
            KERN_INVALID_PROCESSOR_SET => E::InvalidProcessorSet,
            KERN_POLICY_LIMIT => E::PolicyLimit,
            KERN_INVALID_POLICY => E::InvalidPolicy,
            KERN_INVALID_OBJECT => E::InvalidObject,
            KERN_ALREADY_WAITING => E::AlreadyWaiting,
            KERN_DEFAULT_SET => E::DefaultSet,
            KERN_EXCEPTION_PROTECTED => E::ExceptionProtected,
            KERN_INVALID_LEDGER => E::InvalidLedger,
            KERN_INVALID_MEMORY_CONTROL => E::InvalidMemoryControl,
            KERN_INVALID_SECURITY => E::InvalidSecurity,
            KERN_NOT_DEPRESSED => E::NotDepressed,
            KERN_TERMINATED => E::Terminated,
            KERN_LOCK_SET_DESTROYED => E::LockSetDestroyed,
            KERN_LOCK_UNSTABLE => E::LockUnstable,
            KERN_LOCK_OWNED => E::LockOwned,
            KERN_LOCK_OWNED_SELF => E::LockOwnedSelf,
            KERN_SEMAPHORE_DESTROYED => E::SemaphoreDestroyed,
            KERN_RPC_SERVER_TERMINATED => E::RpcServerTerminated,
            KERN_RPC_TERMINATE_ORPHAN => E::RpcTerminateOrphan,
            KERN_RPC_CONTINUE_ORPHAN => E::RpcContinueOrphan,
            KERN_NOT_SUPPORTED => E::NotSupported,
            KERN_NODE_DOWN => E::NodeDown,
            KERN_NOT_WAITING => E::NotWaiting,
            KERN_OPERATION_TIMED_OUT => E::OperationTimedOut,
            _ => E::Unknown,
        }
    }
}

pub struct UsbDevice<'a> {
    inner: NonNull<IOUSBHostDevice>,
    lt: PhantomData<&'a ()>,
}

impl Drop for UsbDevice<'_> {
    fn drop(&mut self) {
        unsafe { self.inner.as_ref().destroy() }
    }
}

#[derive(Default, Clone, Copy)]
pub enum HostObjectInitOptions {
    #[default]
    None = 0,
    DeviceCapture = 1,
}

impl From<HostObjectInitOptions> for IOUSBHostObjectInitOptions {
    fn from(options: HostObjectInitOptions) -> IOUSBHostObjectInitOptions {
        use HostObjectInitOptions as HOIO;
        match options {
            HOIO::None => 0,
            HOIO::DeviceCapture => 1,
        }
    }
}

impl UsbDevice<'_> {
    fn new(
        service: io_service_t,
        options: HostObjectInitOptions,
        queue: &Queue,
    ) -> Result<Self, UsbError> {
        //NOTE: this asks for exclusive access for the device
        //
        //it might be beneficial to use this with IOKit inorder to query without claiming exclusive
        //ownership
        let host_device = IOUSBHostDevice::alloc();
        let mut err = NSErr::new();
        let dev = unsafe {
            host_device.initWithIOService_options_queue_error_interestHandler_(
                service,
                options.into(),
                queue.inner.clone(),
                &mut *err,
                0 as *mut c_void,
            )
        };
        if err.is_err() {
            return Err(err.into());
        }
        //SAFETY: it shouldnt fail here as we already validated the pointer and ensured there was
        //no error with initWithIOService
        let ptr = unsafe { NonNull::new_unchecked(dev as *mut IOUSBHostDevice) };

        Ok(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn send_device_request_with_data(
        &self,
        request: DeviceRequest,
        data: &[u8],
    ) -> Result<u64, UsbError> {
        let data = MutData::with_data(data).raw();
        let mut err = NSErr::new();
        let mut transferred = 0;
        if !unsafe {
            self.inner
                .as_ref()
                .sendDeviceRequest_data_bytesTransferred_completionTimeout_error_(
                    request.into(),
                    data,
                    &mut transferred,
                    0.0,
                    &mut *err,
                )
        } {
            Err(err.into())
        } else {
            Ok(transferred)
        }
    }

    pub fn send_device_request(&self, request: DeviceRequest) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .sendDeviceRequest_error_(request.into(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub async fn enqueue_device_request_with_data(
        &self,
        request: DeviceRequest,
        data: &[u8],
    ) -> Result<(), UsbError> {
        let handler = AsyncDataHandler::new(self.inner, data, |dev, data, cb| {
            let cb = unsafe { downcast_tait(cb) };

            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueDeviceRequest_data_completionTimeout_error_completionHandler_(
                    request.into(),
                    data,
                    0.0,
                    &mut *err,
                    cb,
                )
            } {
                Some(err.into())
            } else {
                None
            }
        });

        handler.await
    }

    pub async fn enqueue_device_request(&self, request: DeviceRequest) -> Result<(), UsbError> {
        let handler = AsyncHandler::new(self.inner, |dev, cb| {
            let cb = unsafe { downcast_tait(cb) };
            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueDeviceRequest_error_completionHandler_(request.into(), &mut *err, cb)
            } {
                Some(err.into())
            } else {
                None
            }
        });
        handler.await
    }

    pub fn string_descriptor(
        &self,
        index: u64,
        language_id: Option<u64>,
    ) -> Result<NSString, UsbError> {
        let mut err = NSErr::new();
        let descriptor = unsafe {
            match language_id {
                Some(id) => self
                    .inner
                    .as_ref()
                    .stringWithIndex_languageID_error_(index, id, &mut *err),
                _ => self.inner.as_ref().stringWithIndex_error_(index, &mut *err),
            }
        };

        if err.is_err() {
            Err(err.into())
        } else {
            Ok(descriptor)
        }
    }

    //returns the current frame number, but also updates the host time aligned with the time which
    //the frame number was last updated
    pub fn frame_number(&self, time: &mut HostTime) -> u64 {
        unsafe { self.inner.as_ref().frameNumberWithTime_(&mut time.inner) }
    }

    pub fn io_data(&self, capacity: u64) -> Result<NSMutableData, UsbError> {
        let mut err = NSErr::new();
        let data = unsafe {
            self.inner
                .as_ref()
                .ioDataWithCapacity_error_(capacity, &mut *err)
        };
        if err.is_err() {
            Err(err.into())
        } else {
            Ok(data)
        }
    }

    pub fn abort_device_requests(&self, abort_option: AbortOption) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .abortDeviceRequestsWithOption_error_(abort_option.into(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn interfaces(
        &self,
        options: HostObjectInitOptions,
    ) -> Option<impl Iterator<Item = HostInterface<'_>>> {
        let current_descriptor = ptr::null();
        Some(Interfaces {
            options,
            queue: self.queue(),
            current_descriptor,
            config_descriptor: unsafe { self.configuration_descriptor()?.inner.as_ref() },
            lt: PhantomData,
        })
    }

    pub fn get_interface(&self, interface_number: u8) -> Option<InterfaceDescriptor<'_>> {
        self.interface_descriptors()?
            .find(|interface| interface.interface_number() == interface_number)
    }

    pub fn get_interface_by_value(&self, interface_number: u8) -> Option<InterfaceDescriptor<'_>> {
        self.interface_descriptors()?
            .find(|interface| interface.interface_number() == interface_number)
    }

    pub fn interface_descriptors(&self) -> Option<impl Iterator<Item = InterfaceDescriptor<'_>>> {
        let current_descriptor = ptr::null();
        Some(InterfaceDescriptors {
            current_descriptor,
            config_descriptor: unsafe { self.configuration_descriptor()?.inner.as_ref() },
            lt: PhantomData,
        })
    }

    pub fn interface_association_descriptors(
        &self,
    ) -> Option<impl Iterator<Item = InterfaceAssociationDescriptor<'_>>> {
        let current_descriptor = ptr::null();
        Some(InterfaceAssociationDescriptors {
            current_descriptor,
            config_descriptor: unsafe { self.configuration_descriptor()?.inner.as_ref() },
            lt: PhantomData,
        })
    }

    pub fn descriptors(&self) -> Option<impl Iterator<Item = DescriptorHeader<'_>>> {
        let current_descriptor = ptr::null();
        Some(Descriptors {
            config_descriptor: unsafe { self.configuration_descriptor()?.inner.as_ref() },
            current_descriptor,
            lt: PhantomData,
        })
    }

    pub fn descriptors_with_type(
        &self,
        descriptor_type: u8,
    ) -> Option<impl Iterator<Item = DescriptorHeader<'_>>> {
        let current_descriptor = ptr::null();
        Some(TypedDescriptors {
            descriptor_type,
            config_descriptor: unsafe { self.configuration_descriptor()?.inner.as_ref() },
            current_descriptor,
            lt: PhantomData,
        })
    }

    pub fn associated_descriptors(
        &self,
        descriptor: &DescriptorHeader<'_>,
    ) -> Option<impl Iterator<Item = DescriptorHeader<'_>>> {
        let current_descriptor = ptr::null();
        Some(AssociatedDescriptors {
            assoc_descriptor: unsafe { descriptor.inner.as_ref() },
            config_descriptor: unsafe { self.configuration_descriptor()?.inner.as_ref() },
            current_descriptor,
            lt: PhantomData,
        })
    }

    pub fn associated_descriptors_with_type(
        &self,
        descriptor: &DescriptorHeader<'_>,
        descriptor_type: u8,
    ) -> Option<impl Iterator<Item = DescriptorHeader<'_>>> {
        let current_descriptor = ptr::null();
        Some(TypedAssociatedDescriptors {
            assoc_descriptor: unsafe { descriptor.inner.as_ref() },
            config_descriptor: unsafe { self.configuration_descriptor()?.inner.as_ref() },
            current_descriptor,
            descriptor_type,
            lt: PhantomData,
        })
    }

    pub fn io_service(&self) -> IoService {
        IoService::from_raw(unsafe { self.inner.as_ref().ioService() })
    }

    pub fn queue(&self) -> Queue {
        Queue::new(unsafe { self.inner.as_ref().queue() })
    }

    pub fn devices<'a, const N: usize>(
        vendor_id: Option<u16>,
        product_id: Option<u16>,
        bcd_device: Option<u16>,
        device_class: Option<u8>,
        device_subclass: Option<u8>,
        device_protocol: Option<u8>,
        speed: Option<u16>, /*, product_ids: Option<[u16; N]>*/
        options: HostObjectInitOptions,
    ) -> Result<impl Iterator<Item = UsbDevice<'a>>, UsbError> {
        let dict = Self::create_matching_dictionary(
            vendor_id,
            product_id,
            bcd_device,
            device_class,
            device_subclass,
            device_protocol,
            speed, /* product_ids */
        )?;

        let mut iter = 0;

        let err = unsafe { IOServiceGetMatchingServices(kIOMasterPortDefault, dict, &mut iter) };

        if err != 0 {
            //uh oh...
        }

        let label = &0;
        let attr = NSObject(ptr::null_mut());

        let queue = Queue::new(unsafe { dispatch_queue_create(label, attr) });

        Ok(Devices {
            queue,
            inner: iter,
            options,
            lt: PhantomData,
        })
    }

    fn create_matching_dictionary(
        vendor_id: Option<u16>,
        product_id: Option<u16>,
        bcd_device: Option<u16>,
        device_class: Option<u8>,
        device_subclass: Option<u8>,
        device_protocol: Option<u8>,
        speed: Option<u16>, /*, product_ids: Option<[u16; N]>*/
    ) -> Result<CFMutableDictionaryRef, UsbError> {
        let vendor_id: NSNum = vendor_id.into();
        let product_id: NSNum = product_id.into();
        let bcd_device: NSNum = bcd_device.into();
        let device_class: NSNum = device_class.into();
        let device_subclass: NSNum = device_subclass.into();
        let device_protocol: NSNum = device_protocol.into();
        let speed: NSNum = speed.into();

        let dict = unsafe {
            IOUSBHostDevice::createMatchingDictionaryWithVendorID_productID_bcdDevice_deviceClass_deviceSubclass_deviceProtocol_speed_productIDArray_(
            vendor_id.into(),
            product_id.into(),
            bcd_device.into(),
            device_class.into(),
            device_subclass.into(),
            device_protocol.into(),
            speed.into(),
            NSArray(ptr::null_mut())
        )
        };

        if dict.is_null() {
            //uh oh...
        }

        Ok(dict)
    }

    pub fn device<const N: usize>(
        vendor_id: Option<u16>,
        product_id: Option<u16>,
        bcd_device: Option<u16>,
        device_class: Option<u8>,
        device_subclass: Option<u8>,
        device_protocol: Option<u8>,
        speed: Option<u16>, /*, product_ids: Option<[u16; N]>*/
        options: HostObjectInitOptions,
    ) -> Result<Self, UsbError> {
        let dict = Self::create_matching_dictionary(
            vendor_id,
            product_id,
            bcd_device,
            device_class,
            device_subclass,
            device_protocol,
            speed, /* product_ids */
        )?;
        let service = unsafe { IOServiceGetMatchingService(kIOMasterPortDefault, dict) };
        let label = &0;
        let attr = NSObject(ptr::null_mut());

        let queue = Queue::new(unsafe { dispatch_queue_create(label, attr) });
        Self::new(service, options, &queue)
    }

    pub fn reset(&self) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe { self.inner.as_ref().resetWithError_(&mut *err) } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn configure(&self, val: u64, match_interfaces: Option<bool>) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        let is_err = unsafe {
            match match_interfaces {
                Some(mtch) => self
                    .inner
                    .as_ref()
                    .configureWithValue_matchInterfaces_error_(val, mtch, &mut *err),
                None => self
                    .inner
                    .as_ref()
                    .configureWithValue_error_(val, &mut *err),
            }
        };
        if is_err {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn device_descriptor(&self) -> Option<DeviceDescriptor<'_>> {
        let ptr = unsafe { self.inner.as_ref().deviceDescriptor() };
        DeviceDescriptor::new(ptr)
    }

    pub fn capability_descriptors(&self) -> impl Iterator<Item = CapabilityDescriptor<'_>> {
        let ptr = unsafe { self.inner.as_ref().capabilityDescriptors() };
        CapabilityDescriptors {
            inner: ptr,
            lt: PhantomData,
        }
    }

    pub fn configuration_descriptor(&self) -> Option<ConfigurationDescriptor<'_>> {
        let ptr = unsafe { self.inner.as_ref().configurationDescriptor() };
        ConfigurationDescriptor::new(ptr)
    }

    pub fn device_address(&self) -> u64 {
        unsafe { self.inner.as_ref().deviceAddress() }
    }
}

pub struct Queue {
    inner: dispatch_queue_t,
}

impl Queue {
    fn new(queue: dispatch_queue_t) -> Self {
        Self { inner: queue }
    }
}

struct Devices<'a> {
    inner: io_service_t,
    queue: Queue,
    options: HostObjectInitOptions,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for Devices<'a> {
    type Item = UsbDevice<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if unsafe { IOIteratorIsValid(self.inner) } == 0 {
            match UsbDevice::new(self.inner, self.options, &self.queue) {
                Ok(dev) => {
                    let next = unsafe { IOIteratorNext(self.inner) };
                    self.inner = next;
                    Some(dev)
                }
                Err(e) => {
                    println!("unexpected err when enumerating devices: {:?}", e);
                    None
                }
            }
        } else {
            None
        }
    }
}

pub struct HostPipe<'a> {
    inner: NonNull<IOUSBHostPipe>,
    lt: PhantomData<&'a ()>,
}

impl HostPipe<'_> {
    fn new(ptr: *const IOUSBHostPipe) -> Self {
        let ptr = unsafe { NonNull::new_unchecked(ptr as *mut IOUSBHostPipe) };
        Self {
            inner: ptr,
            lt: PhantomData,
        }
    }

    #[allow(private_bounds)]
    pub fn adjust(&self, descriptors: impl IntoRawSource) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .adjustPipeWithDescriptors_error_(descriptors.raw(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn set_idle_timeout(&self, duration: f64) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .setIdleTimeout_error_(duration, &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn clear_stall(&self) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe { self.inner.as_ref().clearStallWithError_(&mut *err) } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn send_control_request_with_data(
        &self,
        request: DeviceRequest,
        data: &mut [u8],
    ) -> Result<u64, UsbError> {
        let data = MutData::with_data(data).raw();
        let mut err = NSErr::new();
        let mut transferred = 0;
        if !unsafe {
            self.inner
                .as_ref()
                .sendControlRequest_data_bytesTransferred_completionTimeout_error_(
                    request.into(),
                    data,
                    &mut transferred,
                    0.0,
                    &mut *err,
                )
        } {
            Err(err.into())
        } else {
            Ok(transferred)
        }
    }

    pub fn send_control_request(&self, request: DeviceRequest) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .sendControlRequest_error_(request.into(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub async fn enqueue_control_request_with_data(
        &self,
        request: DeviceRequest,
        data: &mut [u8],
    ) -> Result<(), UsbError> {
        let handler = AsyncDataHandler::new(self.inner, data, |dev, data, cb| {
            let cb = unsafe { downcast_tait(cb) };

            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueControlRequest_data_completionTimeout_error_completionHandler_(
                    request.into(),
                    data,
                    0.0,
                    &mut *err,
                    cb,
                )
            } {
                Some(err.into())
            } else {
                None
            }
        });

        handler.await
    }

    pub async fn enqueue_control_request(&self, request: DeviceRequest) -> Result<(), UsbError> {
        let handler = AsyncHandler::new(self.inner, |dev, cb| {
            let cb = unsafe { downcast_tait(cb) };
            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueControlRequest_error_completionHandler_(request.into(), &mut *err, cb)
            } {
                Some(err.into())
            } else {
                None
            }
        });
        handler.await
    }

    pub fn send_io_request(&self, data: &[u8]) -> Result<u64, UsbError> {
        let mut err = NSErr::new();
        let data = MutData::with_data(data).raw();
        let mut transferred = 0;
        if !unsafe {
            self.inner
                .as_ref()
                .sendIORequestWithData_bytesTransferred_completionTimeout_error_(
                    data,
                    &mut transferred,
                    0.0,
                    &mut *err,
                )
        } {
            Err(err.into())
        } else {
            Ok(transferred)
        }
    }

    pub async fn enqueue_io_request(&self, data: &[u8]) -> Result<(), UsbError> {
        let handler = AsyncDataHandler::new(self.inner, data, |dev, data, cb| {
            let cb = unsafe { downcast_tait(cb) };

            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueIORequestWithData_completionTimeout_error_completionHandler_(
                    data, 0.0, &mut *err, cb,
                )
            } {
                Some(err.into())
            } else {
                None
            }
        });

        handler.await
    }

    pub async fn enqueue_io_request_isochronous_frame(
        &self,
        data: &[u8],
        frames: &mut [IsochronousFrame],
        first_frame_number: u64,
    ) -> Result<(), UsbError> {
        let handler = AsyncDataHandler::new(self.inner, data, |dev, data, cb| {
            let cb = unsafe { downcast_tait(cb) };

            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueIORequestWithData_frameList_frameListCount_firstFrameNumber_error_completionHandler_(
                    data,
                    frames.as_ptr() as *mut IOUSBHostIsochronousFrame,
                    frames.len() as u64,
                    first_frame_number,
                    &mut *err,
                    cb,
                )
            } {
                Some(err.into())
            } else {
                None
            }
        });

        handler.await
    }

    pub async fn enqueue_io_request_isochronous_transaction(
        &self,
        data: &[u8],
        transactions: &mut [IsochronousTransaction],
        first_frame_number: u64,
        options: IsochronousTransactionOptions,
    ) -> Result<(), UsbError> {
        let handler = AsyncDataHandler::new(self.inner, data, |dev, data, cb| {
            let cb = unsafe { downcast_tait(cb) };

            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueIORequestWithData_transactionList_transactionListCount_firstFrameNumber_options_error_completionHandler_(
                    data,
                    transactions.as_ptr() as *mut IOUSBHostIsochronousTransaction,
                    transactions.len() as u64,
                    first_frame_number,
                    options.into(),
                    &mut *err,
                    cb,
                )
            } {
                Some(err.into())
            } else {
                None
            }
        });

        handler.await
    }

    pub fn send_io_request_isochronous_frame(
        &self,
        data: &[u8],
        frames: &mut [IsochronousFrame],
        first_frame_number: u64,
    ) -> Result<(), UsbError> {
        let data = MutData::with_data(data).raw();
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .sendIORequestWithData_frameList_frameListCount_firstFrameNumber_error_(
                    data,
                    frames.as_ptr() as *mut IOUSBHostIsochronousFrame,
                    frames.len() as u64,
                    first_frame_number,
                    &mut *err,
                )
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn send_io_request_isochronous_transaction(
        &self,
        data: &[u8],
        transactions: &mut [IsochronousTransaction],
        first_frame_number: u64,
        options: IsochronousTransactionOptions,
    ) -> Result<(), UsbError> {
        let data = MutData::with_data(data).raw();
        let mut err = NSErr::new();
        if !unsafe {
            self.inner.as_ref().sendIORequestWithData_transactionList_transactionListCount_firstFrameNumber_options_error_(data, transactions.as_ptr() as *mut IOUSBHostIsochronousTransaction, transactions.len() as u64, first_frame_number, options.into(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn abort(&self, abort: AbortOption) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .abortWithOption_error_(abort.into(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn enable_streams(&self) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe { self.inner.as_ref().enableStreamsWithError_(&mut *err) } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn disable_streams(&self) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe { self.inner.as_ref().disableStreamsWithError_(&mut *err) } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn copy_stream(&self, stream_id: u64) -> Result<HostStream, UsbError> {
        let mut err = NSErr::new();
        let stream = unsafe {
            self.inner
                .as_ref()
                .copyStreamWithStreamID_error_(stream_id, &mut *err)
        };
        if err.is_err() {
            Err(err.into())
        } else {
            Ok(HostStream { inner: stream })
        }
    }

    #[allow(private_interfaces)]
    pub fn original_descriptors(
        &self,
    ) -> impl Iterator<Item = IoSourceDescriptor<'_>> + IntoRawSource {
        let ptr = unsafe { self.inner.as_ref().originalDescriptors() };
        IoSourceDescriptors {
            inner: ptr,
            lt: PhantomData,
        }
    }

    #[allow(private_interfaces)]
    pub fn descriptors(&self) -> impl Iterator<Item = IoSourceDescriptor<'_>> + IntoRawSource {
        let ptr = unsafe { self.inner.as_ref().descriptors() };
        IoSourceDescriptors {
            inner: ptr,
            lt: PhantomData,
        }
    }

    pub fn idle_timeout(&self) -> f64 {
        unsafe { self.inner.as_ref().idleTimeout() }
    }
}

pub struct HostStream {
    inner: IOUSBHostStream,
}

impl HostStream {
    pub fn abort(&self, option: AbortOption) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe { self.inner.abortWithOption_error_(option.into(), &mut *err) } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn send_io_request(&self, data: &mut [u8]) -> Result<u64, UsbError> {
        let mut err = NSErr::new();
        let data = MutData::with_data(data).raw();
        let mut transferred = 0;
        if !unsafe {
            self.inner.sendIORequestWithData_bytesTransferred_error_(
                data,
                &mut transferred,
                &mut *err,
            )
        } {
            Err(err.into())
        } else {
            Ok(transferred)
        }
    }

    pub async fn enqueue_io_request(&self, data: &[u8]) -> Result<(), UsbError> {
        let ptr = unsafe {
            NonNull::new_unchecked(&self.inner as *const IOUSBHostStream as *mut IOUSBHostStream)
        };
        let handler = AsyncDataHandler::new(ptr, data, |dev, data, cb| {
            let cb = unsafe { downcast_tait(cb) };

            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueIORequestWithData_error_completionHandler_(data, &mut *err, cb)
            } {
                Some(err.into())
            } else {
                None
            }
        });

        handler.await
    }

    /*
    fn host_pipe(&self) -> HostPipe {
        HostPipe{
            inner: unsafe { self.inner.hostPipe() }
        }
    }
    */

    pub fn stream_id(&self) -> u64 {
        unsafe { self.inner.streamID() }
    }
}

pub struct HostIoSource {
    inner: IOUSBHostIOSource,
}

/*
pub struct InterfacePropertyKey(NSString);
pub struct DevicePropertyKey(NSString);
pub struct MatchingPropertyKey(NSString);
pub struct PropertyKey(NSString);
*/

impl HostIoSource {
    /*
    fn host_interface(&self) -> HostInterface {
        HostInterface {
            inner: unsafe{self.inner.hostInterface()}
        }
    }
    */

    pub fn device_address(&self) -> u64 {
        unsafe { self.inner.deviceAddress() }
    }

    pub fn endpoint_address(&self) -> u64 {
        unsafe { self.inner.endpointAddress() }
    }
}

pub struct IoSourceDescriptors<'a> {
    inner: *const IOUSBHostIOSourceDescriptors,
    lt: PhantomData<&'a IOUSBHostIOSourceDescriptors>,
}

impl IntoRawSource for IoSourceDescriptors<'_> {
    fn raw(&self) -> *const IOUSBHostIOSourceDescriptors {
        self.inner
    }
}

impl<'a> Iterator for IoSourceDescriptors<'a> {
    type Item = IoSourceDescriptor<'a>;
    fn next(&mut self) -> Option<IoSourceDescriptor<'a>> {
        let desc = IoSourceDescriptor::new(self.inner)?;
        self.inner = unsafe { self.inner.add(1) };
        Some(desc)
    }
}

pub struct IoSourceDescriptor<'a> {
    inner: NonNull<IOUSBHostIOSourceDescriptors>,
    lt: PhantomData<&'a ()>,
}

trait IntoRawSource {
    fn raw(&self) -> *const IOUSBHostIOSourceDescriptors;
}

impl IntoRawSource for IoSourceDescriptor<'_> {
    fn raw(&self) -> *const IOUSBHostIOSourceDescriptors {
        let ptr = unsafe { self.inner.as_ref() as *const IOUSBHostIOSourceDescriptors };
        ptr
    }
}

impl IoSourceDescriptor<'_> {
    fn new(ptr: *const IOUSBHostIOSourceDescriptors) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBHostIOSourceDescriptors)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn bcd_usb(&self) -> u16 {
        unsafe { self.inner.as_ref().bcdUSB }
    }

    pub fn endpoint_descriptor(&self) -> EndpointDescriptor<'_> {
        let ptr = &unsafe { self.inner.as_ref().descriptor };
        EndpointDescriptor::new(ptr).unwrap()
    }

    pub fn super_speed_companion_descriptor(&self) -> SuperSpeedCompanionDescriptor {
        let ptr = &unsafe { self.inner.as_ref().ssCompanionDescriptor };
        SuperSpeedCompanionDescriptor::new(ptr).unwrap()
    }

    pub fn super_speed_plus_companion_descriptor(&self) -> SuperSpeedPlusCompanionDescriptor {
        let ptr = &unsafe { self.inner.as_ref().sspCompanionDescriptor };
        SuperSpeedPlusCompanionDescriptor::new(ptr).unwrap()
    }
}

pub struct SuperSpeedCompanionDescriptor<'a> {
    inner: NonNull<IOUSBSuperSpeedEndpointCompanionDescriptor>,
    lt: PhantomData<&'a IOUSBSuperSpeedEndpointCompanionDescriptor>,
}

pub struct SuperSpeedPlusCompanionDescriptor<'a> {
    inner: NonNull<IOUSBSuperSpeedPlusIsochronousEndpointCompanionDescriptor>,
    lt: PhantomData<&'a IOUSBSuperSpeedPlusIsochronousEndpointCompanionDescriptor>,
}

impl SuperSpeedPlusCompanionDescriptor<'_> {
    fn new(ptr: *const IOUSBSuperSpeedPlusIsochronousEndpointCompanionDescriptor) -> Option<Self> {
        let ptr =
            NonNull::new(ptr as *mut IOUSBSuperSpeedPlusIsochronousEndpointCompanionDescriptor)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }
    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn bytes_per_interval(&self) -> u32 {
        unsafe { self.inner.as_ref().dwBytesPerInterval }
    }
}

impl SuperSpeedCompanionDescriptor<'_> {
    fn new(ptr: *const IOUSBSuperSpeedEndpointCompanionDescriptor) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBSuperSpeedEndpointCompanionDescriptor)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn max_burst(&self) -> u8 {
        unsafe { self.inner.as_ref().bMaxBurst }
    }

    pub fn attributes(&self) -> u8 {
        unsafe { self.inner.as_ref().bmAttributes }
    }

    pub fn bytes_per_interval(&self) -> u16 {
        unsafe { self.inner.as_ref().wBytesPerInterval }
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum DescriptorType {
    Device = 1,
    Configuration = 2,
    String = 3,
    Interface = 4,
    Endpoint = 5,
    DeviceQualifier = 6,
    OtherSpeedConfig = 7,
    InterfacePower = 8,
    OTG = 9,
    Debug = 10,
    InterfaceAssociation = 11,
    CapabilityDescriptor = 15,
    DeviceCapability = 16,
    HID = 33,
    Report = 34,
    Physical = 35,
    Hub = 41,
    SuperSpeedHub = 42,
    SuperSpeedEndpointCompanion = 48,
    SuperSpeedPlusIsochronousEndpointCompanion = 49,
    Other(u8),
}

impl From<u8> for DescriptorType {
    fn from(num: u8) -> DescriptorType {
        use DescriptorType as DT;
        match num {
            1 => DT::Device,
            2 => DT::Configuration,
            3 => DT::String,
            4 => DT::Interface,
            5 => DT::Endpoint,
            6 => DT::DeviceQualifier,
            7 => DT::OtherSpeedConfig,
            8 => DT::InterfacePower,
            9 => DT::OTG,
            10 => DT::Debug,
            11 => DT::InterfaceAssociation,
            15 => DT::CapabilityDescriptor,
            16 => DT::DeviceCapability,
            33 => DT::HID,
            34 => DT::Report,
            35 => DT::Physical,
            41 => DT::Hub,
            42 => DT::SuperSpeedHub,
            48 => DT::SuperSpeedEndpointCompanion,
            49 => DT::SuperSpeedPlusIsochronousEndpointCompanion,
            other => DT::Other(other),
        }
    }
}

impl From<DescriptorType> for u8 {
    fn from(desc: DescriptorType) -> u8 {
        use DescriptorType as DT;
        match desc {
            DT::Device => 1,
            DT::Configuration => 2,
            DT::String => 3,
            DT::Interface => 4,
            DT::Endpoint => 5,
            DT::DeviceQualifier => 6,
            DT::OtherSpeedConfig => 7,
            DT::InterfacePower => 8,
            DT::OTG => 9,
            DT::Debug => 10,
            DT::InterfaceAssociation => 11,
            DT::CapabilityDescriptor => 15,
            DT::DeviceCapability => 16,
            DT::HID => 33,
            DT::Report => 34,
            DT::Physical => 35,
            DT::Hub => 41,
            DT::SuperSpeedHub => 42,
            DT::SuperSpeedEndpointCompanion => 48,
            DT::SuperSpeedPlusIsochronousEndpointCompanion => 49,
            DT::Other(o) => o,
        }
    }
}

impl From<DeviceRequest> for IOUSBDeviceRequest {
    fn from(req: DeviceRequest) -> IOUSBDeviceRequest {
        req.inner
    }
}

#[derive(Clone, Copy)]
pub struct DeviceRequest {
    inner: IOUSBDeviceRequest,
}

impl DeviceRequest {
    pub fn new(
        request_type: DeviceRequestType,
        request: u8,
        value: u16,
        index: u16,
        length: u16,
    ) -> Self {
        let inner = IOUSBDeviceRequest {
            bmRequestType: request_type.into(),
            bRequest: request,
            wValue: value,
            wIndex: index,
            wLength: length,
        };
        Self { inner }
    }

    pub fn request_type(&self) -> u8 {
        self.inner.bmRequestType
    }

    pub fn request(&self) -> u8 {
        self.inner.bRequest
    }

    pub fn value(&self) -> u16 {
        self.inner.wValue
    }

    pub fn index(&self) -> u16 {
        self.inner.wIndex
    }

    pub fn length(&self) -> u16 {
        self.inner.wLength
    }
}

pub struct HostInterface<'a> {
    inner: NonNull<IOUSBHostInterface>,
    lt: PhantomData<&'a ()>,
}

impl HostInterface<'_> {
    fn new(ptr: *const IOUSBHostInterface) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBHostInterface)?;
        Some(HostInterface {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn idle_timeout(&self) -> f64 {
        unsafe { self.inner.as_ref().idleTimeout() }
    }

    pub fn set_idle_timeout(&self, interval: f64) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if unsafe {
            !self
                .inner
                .as_ref()
                .setIdleTimeout_error_(interval, &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn configuration_descriptor(&self) -> Option<ConfigurationDescriptor<'_>> {
        let ptr = unsafe { self.inner.as_ref().configurationDescriptor() };
        ConfigurationDescriptor::new(ptr)
    }

    pub fn interface_descriptor(&self) -> Option<InterfaceDescriptor<'_>> {
        let ptr = unsafe { self.inner.as_ref().interfaceDescriptor() };
        InterfaceDescriptor::new(ptr)
    }

    pub fn create_matching_dictionary<const N: usize>(
        vendor_id: Option<u16>,
        product_id: Option<u16>,
        bcd_device: Option<u16>,
        interface_number: Option<u8>,
        configuration_value: Option<u8>,
        interface_class: Option<u8>,
        interface_subclass: Option<u8>,
        interface_protocol: Option<u8>,
        speed: Option<u16>, /*product_ids: Option<[u16; N]>*/
    ) -> Result<CFMutableDictionaryRef, UsbError> {
        let vendor_id: NSNum = vendor_id.into();
        let product_id: NSNum = product_id.into();
        let bcd_device: NSNum = bcd_device.into();
        let interface_number: NSNum = interface_number.into();
        let configuration_value: NSNum = configuration_value.into();
        let interface_class: NSNum = interface_class.into();
        let interface_subclass: NSNum = interface_subclass.into();
        let interface_protocol: NSNum = interface_protocol.into();
        let speed: NSNum = speed.into();

        let dict = unsafe {
            IOUSBHostInterface::createMatchingDictionaryWithVendorID_productID_bcdDevice_interfaceNumber_configurationValue_interfaceClass_interfaceSubclass_interfaceProtocol_speed_productIDArray_(
            vendor_id.into(),
            product_id.into(),
            bcd_device.into(),
            interface_number.into(),
            configuration_value.into(),
            interface_class.into(),
            interface_subclass.into(),
            interface_protocol.into(),
            speed.into(),
            NSArray(ptr::null_mut()),
        )
        };

        if dict.is_null() {
            //uh oh...
        }
        Ok(dict)
    }

    pub fn endpoint_descriptors(&self) -> Option<impl Iterator<Item = EndpointDescriptor<'_>>> {
        let config_descriptor = unsafe { self.configuration_descriptor()?.inner.as_ref() };
        let interface_descriptor = unsafe { self.interface_descriptor()?.inner.as_ref() };
        let current_descriptor = ptr::null();
        Some(EndpointDescriptors {
            config_descriptor,
            interface_descriptor,
            current_descriptor,
            lt: PhantomData,
        })
    }

    pub fn pipes(&self) -> Option<impl Iterator<Item = HostPipe<'_>>> {
        let config_descriptor = unsafe { self.configuration_descriptor()?.inner.as_ref() };
        let interface_descriptor = unsafe { self.interface_descriptor()?.inner.as_ref() };
        let current_descriptor = ptr::null();
        Some(Pipes {
            config_descriptor,
            interface_descriptor,
            current_descriptor,
            interface: &self,
        })
    }

    pub fn select_alternate_setting(&self, alternate_setting: u8) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .selectAlternateSetting_error_(alternate_setting as u64, &mut *err)
        } {
            return Err(err.into());
        } else {
            Ok(())
        }
    }

    pub fn copy_pipe(&self, address: u64) -> Result<HostPipe<'_>, UsbError> {
        let mut err = NSErr::new();
        let pipe = unsafe {
            self.inner
                .as_ref()
                .copyPipeWithAddress_error_(address, &mut *err)
        };

        if err.is_err() {
            return Err(err.into());
        } else {
            Ok(HostPipe::new(&pipe))
        }
    }
}

pub struct Pipes<'a> {
    interface: &'a HostInterface<'a>,
    config_descriptor: *const IOUSBConfigurationDescriptor,
    interface_descriptor: *const IOUSBInterfaceDescriptor,
    current_descriptor: *const IOUSBDescriptorHeader,
}

impl<'a> Iterator for Pipes<'a> {
    type Item = HostPipe<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextEndpointDescriptor(
                self.config_descriptor,
                self.interface_descriptor,
                self.current_descriptor,
            )
        };
        if next.is_null() {
            return None;
        }
        self.current_descriptor = next as *const IOUSBDescriptorHeader;

        match self
            .interface
            .copy_pipe(unsafe { (*next).bEndpointAddress } as u64)
        {
            Ok(pipe) => Some(pipe),
            Err(e) => {
                println!("err while enumerating pipes: {:?}", e);
                None
            }
        }
    }
}

/*
pub struct NSArr(NSArray);

impl<T, const N: usize> From<Option<[T; N]>> for NSArr {
    fn from(arr: Option<[T; N]>) -> NSArr {
        let ptr = if let Some(arr) = arr {
            let alloc = NSMutableArray::alloc();
            //alloc.initWithObjects_count_();

            todo!()
        } else {
            NSArray(ptr::null_mut())
        };
        todo!()
    }
}
*/

pub struct NSNum(NSNumber);

impl From<Option<u16>> for NSNum {
    fn from(opt: Option<u16>) -> NSNum {
        NSNum(if let Some(num) = opt {
            let alloc = NSNumber::alloc();
            unsafe { alloc.initWithUnsignedShort_(num) }
        } else {
            NSNumber(ptr::null_mut())
        })
    }
}

impl From<Option<u8>> for NSNum {
    fn from(opt: Option<u8>) -> NSNum {
        NSNum(if let Some(num) = opt {
            let alloc = NSNumber::alloc();
            unsafe { alloc.initWithUnsignedChar_(num) }
        } else {
            NSNumber(ptr::null_mut())
        })
    }
}

impl From<NSNum> for NSNumber {
    fn from(f: NSNum) -> NSNumber {
        f.0
    }
}

pub struct NSErr(NSError);

impl NSErr {
    pub fn new() -> Self {
        Self(NSError(ptr::null_mut()))
    }

    pub fn is_err(&self) -> bool {
        !self.0 .0.is_null()
    }
}

impl From<NSErr> for UsbError {
    fn from(err: NSErr) -> UsbError {
        //NOTE: this is the same as `kern_return_t`
        match unsafe { err.0.code() } {
            _ => todo!(),
        }
    }
}

impl Deref for NSErr {
    type Target = NSError;
    fn deref(&self) -> &NSError {
        &self.0
    }
}

impl DerefMut for NSErr {
    fn deref_mut(&mut self) -> &mut NSError {
        &mut self.0
    }
}

///NOTE: this is commonly referred to as `altsetting`
pub struct InterfaceDescriptor<'a> {
    inner: NonNull<IOUSBInterfaceDescriptor>,
    lt: PhantomData<&'a IOUSBInterfaceDescriptor>,
}

impl InterfaceDescriptor<'_> {
    fn new(ptr: *const IOUSBInterfaceDescriptor) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBInterfaceDescriptor)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn interface_number(&self) -> u8 {
        unsafe { self.inner.as_ref().bInterfaceNumber }
    }

    pub fn alternate_setting(&self) -> u8 {
        unsafe { self.inner.as_ref().bAlternateSetting }
    }

    pub fn endpoint_count(&self) -> u8 {
        unsafe { self.inner.as_ref().bNumEndpoints }
    }

    pub fn interface_class(&self) -> u8 {
        unsafe { self.inner.as_ref().bInterfaceClass }
    }

    pub fn interface_subclass(&self) -> u8 {
        unsafe { self.inner.as_ref().bInterfaceSubClass }
    }

    pub fn interface_protocol(&self) -> u8 {
        unsafe { self.inner.as_ref().bInterfaceProtocol }
    }

    pub fn interface(&self) -> u8 {
        unsafe { self.inner.as_ref().iInterface }
    }
}

pub struct DeviceDescriptor<'a> {
    inner: NonNull<IOUSBDeviceDescriptor>,
    lt: PhantomData<&'a IOUSBDeviceDescriptor>,
}

impl Drop for DeviceDescriptor<'_> {
    fn drop(&mut self) {}
}

impl DeviceDescriptor<'_> {
    fn new(ptr: *const IOUSBDeviceDescriptor) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBDeviceDescriptor)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn bcd_usb(&self) -> u16 {
        unsafe { self.inner.as_ref().bcdUSB }
    }

    pub fn device_class(&self) -> u8 {
        unsafe { self.inner.as_ref().bDeviceClass }
    }

    pub fn device_subclass(&self) -> u8 {
        unsafe { self.inner.as_ref().bDeviceSubClass }
    }

    pub fn device_protocol(&self) -> u8 {
        unsafe { self.inner.as_ref().bDeviceProtocol }
    }

    pub fn max_packet_size(&self) -> u8 {
        unsafe { self.inner.as_ref().bMaxPacketSize0 }
    }

    pub fn vendor_id(&self) -> u16 {
        unsafe { self.inner.as_ref().idVendor }
    }

    pub fn product_id(&self) -> u16 {
        unsafe { self.inner.as_ref().idProduct }
    }

    pub fn bcd_device(&self) -> u16 {
        unsafe { self.inner.as_ref().bcdDevice }
    }

    pub fn manufacturer(&self) -> u8 {
        unsafe { self.inner.as_ref().iManufacturer }
    }

    pub fn product(&self) -> u8 {
        unsafe { self.inner.as_ref().iProduct }
    }

    pub fn serial_number(&self) -> u8 {
        unsafe { self.inner.as_ref().iSerialNumber }
    }

    pub fn configuration_count(&self) -> u8 {
        unsafe { self.inner.as_ref().bNumConfigurations }
    }
}

pub struct ConfigurationDescriptor<'a> {
    inner: NonNull<IOUSBConfigurationDescriptor>,
    lt: PhantomData<&'a IOUSBConfigurationDescriptor>,
}

impl ConfigurationDescriptor<'_> {
    fn new(ptr: *const IOUSBConfigurationDescriptor) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBConfigurationDescriptor)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn total_length(&self) -> u16 {
        unsafe { self.inner.as_ref().wTotalLength }
    }

    pub fn interface_count(&self) -> u8 {
        unsafe { self.inner.as_ref().bNumInterfaces }
    }

    pub fn configuration_value(&self) -> u8 {
        unsafe { self.inner.as_ref().bConfigurationValue }
    }

    pub fn configuration(&self) -> u8 {
        unsafe { self.inner.as_ref().iConfiguration }
    }

    pub fn attributes(&self) -> u8 {
        unsafe { self.inner.as_ref().bmAttributes }
    }

    pub fn max_power(&self) -> u8 {
        unsafe { self.inner.as_ref().MaxPower }
    }

    pub fn max_power_milliamps(&self, usb_device_speed: u32) -> u32 {
        unsafe { IOUSBGetConfigurationMaxPowerMilliAmps(usb_device_speed, self.inner.as_ref()) }
    }
}

pub struct Descriptors<'a> {
    config_descriptor: *const IOUSBConfigurationDescriptor,
    current_descriptor: *const IOUSBDescriptorHeader,
    lt: PhantomData<&'a ()>,
}

pub struct DescriptorHeader<'a> {
    inner: NonNull<IOUSBDescriptorHeader>,
    lt: PhantomData<&'a ()>,
}

impl DescriptorHeader<'_> {
    fn new(ptr: *const IOUSBDescriptorHeader) -> Self {
        let ptr = unsafe { NonNull::new_unchecked(ptr as *mut IOUSBDescriptorHeader) };
        Self {
            inner: ptr,
            lt: PhantomData,
        }
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }
}

impl<'a> Iterator for Descriptors<'a> {
    type Item = DescriptorHeader<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next =
            unsafe { IOUSBGetNextDescriptor(self.config_descriptor, self.current_descriptor) };
        if next.is_null() {
            return None;
        }
        self.current_descriptor = next;
        Some(DescriptorHeader::new(next))
    }
}

pub struct TypedDescriptors<'a> {
    descriptor_type: u8,
    config_descriptor: *const IOUSBConfigurationDescriptor,
    current_descriptor: *const IOUSBDescriptorHeader,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for TypedDescriptors<'a> {
    type Item = DescriptorHeader<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextDescriptorWithType(
                self.config_descriptor,
                self.current_descriptor,
                self.descriptor_type,
            )
        };
        if next.is_null() {
            return None;
        }
        self.current_descriptor = next;
        Some(DescriptorHeader::new(next))
    }
}

pub struct AssociatedDescriptors<'a> {
    config_descriptor: *const IOUSBConfigurationDescriptor,
    current_descriptor: *const IOUSBDescriptorHeader,
    assoc_descriptor: *const IOUSBDescriptorHeader,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for AssociatedDescriptors<'a> {
    type Item = DescriptorHeader<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextAssociatedDescriptor(
                self.config_descriptor,
                self.assoc_descriptor,
                self.current_descriptor,
            )
        };
        if next.is_null() {
            return None;
        }
        self.current_descriptor = next;
        Some(DescriptorHeader::new(next))
    }
}

pub struct TypedAssociatedDescriptors<'a> {
    descriptor_type: u8,
    config_descriptor: *const IOUSBConfigurationDescriptor,
    current_descriptor: *const IOUSBDescriptorHeader,
    assoc_descriptor: *const IOUSBDescriptorHeader,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for TypedAssociatedDescriptors<'a> {
    type Item = DescriptorHeader<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextAssociatedDescriptorWithType(
                self.config_descriptor,
                self.assoc_descriptor,
                self.current_descriptor,
                self.descriptor_type,
            )
        };
        if next.is_null() {
            return None;
        }
        self.current_descriptor = next;
        Some(DescriptorHeader::new(next))
    }
}

pub struct InterfaceAssociationDescriptors<'a> {
    config_descriptor: *const IOUSBConfigurationDescriptor,
    current_descriptor: *const IOUSBDescriptorHeader,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for InterfaceAssociationDescriptors<'a> {
    type Item = InterfaceAssociationDescriptor<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextInterfaceAssociationDescriptor(
                self.config_descriptor,
                self.current_descriptor,
            )
        };

        if next.is_null() {
            return None;
        }

        self.current_descriptor = next as *const IOUSBDescriptorHeader;
        InterfaceAssociationDescriptor::new(next)
    }
}

pub struct InterfaceAssociationDescriptor<'a> {
    inner: NonNull<IOUSBInterfaceAssociationDescriptor>,
    lt: PhantomData<&'a ()>,
}

impl InterfaceAssociationDescriptor<'_> {
    fn new(raw: *const IOUSBInterfaceAssociationDescriptor) -> Option<Self> {
        let ptr = NonNull::new(raw as *mut IOUSBInterfaceAssociationDescriptor)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn first_interface(&self) -> u8 {
        unsafe { self.inner.as_ref().bFirstInterface }
    }

    pub fn interface_count(&self) -> u8 {
        unsafe { self.inner.as_ref().bInterfaceCount }
    }

    pub fn function_class(&self) -> u8 {
        unsafe { self.inner.as_ref().bFunctionClass }
    }

    pub fn function_subclass(&self) -> u8 {
        unsafe { self.inner.as_ref().bFunctionSubClass }
    }

    pub fn function(&self) -> u8 {
        unsafe { self.inner.as_ref().iFunction }
    }
}

pub struct InterfaceDescriptors<'a> {
    config_descriptor: *const IOUSBConfigurationDescriptor,
    current_descriptor: *const IOUSBDescriptorHeader,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for InterfaceDescriptors<'a> {
    type Item = InterfaceDescriptor<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextInterfaceDescriptor(self.config_descriptor, self.current_descriptor)
        };

        if next.is_null() {
            return None;
        }

        let desc = InterfaceDescriptor::new(next)?;
        self.current_descriptor = next as *const IOUSBDescriptorHeader;
        Some(desc)
    }
}

pub struct Interfaces<'a> {
    config_descriptor: *const IOUSBConfigurationDescriptor,
    current_descriptor: *const IOUSBDescriptorHeader,
    options: HostObjectInitOptions,
    queue: Queue,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for Interfaces<'a> {
    type Item = HostInterface<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextInterfaceDescriptor(self.config_descriptor, self.current_descriptor)
        };

        if next.is_null() {
            return None;
        }

        let vendor_id = 0;
        let product_id = 0;

        match unsafe {
            HostInterface::create_matching_dictionary::<0>(
                Some(vendor_id),
                Some(product_id),
                None,
                Some((*next).bInterfaceNumber),
                Some((*self.config_descriptor).bConfigurationValue),
                Some((*next).bInterfaceClass),
                Some((*next).bInterfaceSubClass),
                Some((*next).bInterfaceProtocol),
                None,
            )
        } {
            Ok(dict) => {
                let service = unsafe { IOServiceGetMatchingService(kIOMasterPortDefault, dict) };

                let mut err = NSErr::new();

                let interface = IOUSBHostInterface::alloc();
                let interface = unsafe {
                    IIOUSBHostInterface::initWithIOService_options_queue_error_interestHandler_(
                        &interface,
                        service,
                        self.options.into(),
                        self.queue.inner,
                        &mut *err,
                        0 as *mut c_void,
                    )
                };

                if err.is_err() {
                    println!("error while enumerating interface descriptors: {:?}", err.0);
                    return None;
                }
                let interface = HostInterface::new(interface as *const IOUSBHostInterface)?;
                self.current_descriptor = next as *const IOUSBDescriptorHeader;
                Some(interface)
            }
            Err(e) => {
                println!("error while enumerating interface descriptors: {:?}", e);
                None
            }
        }
    }
}

pub struct EndpointDescriptors<'a> {
    config_descriptor: *const IOUSBConfigurationDescriptor,
    interface_descriptor: *const IOUSBInterfaceDescriptor,
    current_descriptor: *const IOUSBDescriptorHeader,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for EndpointDescriptors<'a> {
    type Item = EndpointDescriptor<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextEndpointDescriptor(
                self.config_descriptor,
                self.interface_descriptor,
                self.current_descriptor,
            )
        };
        if next.is_null() {
            return None;
        }
        self.current_descriptor = next as *const IOUSBDescriptorHeader;
        EndpointDescriptor::new(next)
    }
}

pub struct CapabilityDescriptor<'a> {
    inner: NonNull<IOUSBBOSDescriptor>,
    lt: PhantomData<&'a IOUSBBOSDescriptor>,
}

impl CapabilityDescriptor<'_> {
    fn new(ptr: *const IOUSBBOSDescriptor) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBBOSDescriptor)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn capabilities(&self) -> impl Iterator<Item = Capability<'_>> {
        let current_descriptor = ptr::null();
        Capabilities {
            current_descriptor,
            bos_descriptor: unsafe { self.inner.as_ref() },
            lt: PhantomData,
        }
    }

    pub fn capabilities_with_type(
        &self,
        capability_type: u8,
    ) -> impl Iterator<Item = Capability<'_>> {
        let current_descriptor = ptr::null();
        TypedCapabilities {
            current_descriptor,
            capability_type,
            bos_descriptor: unsafe { self.inner.as_ref() },
            lt: PhantomData,
        }
    }

    pub fn usb_20_extension_device_capability_descriptor(
        &self,
    ) -> Option<DeviceCapabilityUsb2Extension<'_>> {
        let ptr = unsafe { IOUSBGetUSB20ExtensionDeviceCapabilityDescriptor(self.inner.as_ref()) };
        DeviceCapabilityUsb2Extension::new(ptr)
    }

    pub fn super_speed_device_capability_descriptor(&self) -> Option<DeviceCapabilitySS<'_>> {
        let ptr = unsafe { IOUSBGetSuperSpeedDeviceCapabilityDescriptor(self.inner.as_ref()) };
        DeviceCapabilitySS::new(ptr)
    }

    pub fn super_speed_plus_capability_descriptor(&self) -> Option<DeviceCapabilitySSP<'_>> {
        DeviceCapabilitySSP::new(unsafe {
            IOUSBGetSuperSpeedPlusDeviceCapabilityDescriptor(self.inner.as_ref())
        })
    }

    pub fn container_id_descriptor(&self) -> Option<DeviceCapabilityContainerId<'_>> {
        let ptr = unsafe { IOUSBGetContainerIDDescriptor(self.inner.as_ref()) };
        DeviceCapabilityContainerId::new(ptr)
    }

    pub fn platform_capability_descriptor(
        &self,
        uuid: Option<&str>,
    ) -> Option<PlatformCapabilityDescriptor<'_>> {
        let ptr = unsafe {
            match uuid {
                Some(uuid) => IOUSBGetPlatformCapabilityDescriptorWithUUID(
                    self.inner.as_ref(),
                    uuid.as_bytes().as_ptr() as *mut u8,
                ),
                None => IOUSBGetPlatformCapabilityDescriptor(self.inner.as_ref()),
            }
        };
        PlatformCapabilityDescriptor::new(ptr)
    }

    pub fn billboard_descriptor(&self) -> Option<DeviceCapabilityBillboard> {
        DeviceCapabilityBillboard::new(unsafe { IOUSBGetBillboardDescriptor(self.inner.as_ref()) })
    }
}

pub struct DeviceCapabilityUsb2Extension<'a> {
    inner: NonNull<IOUSBDeviceCapabilityUSB2Extension>,
    lt: PhantomData<&'a ()>,
}

impl DeviceCapabilityUsb2Extension<'_> {
    fn new(ptr: *const IOUSBDeviceCapabilityUSB2Extension) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBDeviceCapabilityUSB2Extension)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn device_capability_type(&self) -> DeviceCapabilityType {
        unsafe { self.inner.as_ref().bDevCapabilityType }.into()
    }

    pub fn attributes(&self) -> u32 {
        unsafe { self.inner.as_ref().bmAttributes }
    }
}

pub struct DeviceCapabilitySS<'a> {
    inner: NonNull<IOUSBDeviceCapabilitySuperSpeedUSB>,
    lt: PhantomData<&'a ()>,
}

impl DeviceCapabilitySS<'_> {
    fn new(ptr: *const IOUSBDeviceCapabilitySuperSpeedUSB) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBDeviceCapabilitySuperSpeedUSB)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn device_capability_type(&self) -> DeviceCapabilityType {
        unsafe { self.inner.as_ref().bDevCapabilityType }.into()
    }

    pub fn attributes(&self) -> u8 {
        unsafe { self.inner.as_ref().bmAttributes }
    }

    pub fn speeds_supported(&self) -> u16 {
        unsafe { self.inner.as_ref().wSpeedsSupported }
    }

    pub fn functionality_support(&self) -> u8 {
        unsafe { self.inner.as_ref().bFunctionalitySupport }
    }

    pub fn u1_dev_exit_lat(&self) -> u8 {
        unsafe { self.inner.as_ref().bU1DevExitLat }
    }

    pub fn u2_dev_exit_lat(&self) -> u16 {
        unsafe { self.inner.as_ref().wU2DevExitLat }
    }

    pub fn dev_exit_lat(&self) -> (u8, u16) {
        let ptr = unsafe { self.inner.as_ref() };
        (ptr.bU1DevExitLat, ptr.wU2DevExitLat)
    }
}

pub struct DeviceCapabilitySSP<'a> {
    inner: NonNull<IOUSBDeviceCapabilitySuperSpeedPlusUSB>,
    lt: PhantomData<&'a ()>,
}

impl DeviceCapabilitySSP<'_> {
    fn new(ptr: *const IOUSBDeviceCapabilitySuperSpeedPlusUSB) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBDeviceCapabilitySuperSpeedPlusUSB)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn device_capability_type(&self) -> DeviceCapabilityType {
        unsafe { self.inner.as_ref().bDevCapabilityType }.into()
    }

    pub fn attributes(&self) -> u32 {
        unsafe { self.inner.as_ref().bmAttributes }
    }

    pub fn functionality_support(&self) -> u16 {
        unsafe { self.inner.as_ref().wFunctionalitySupport }
    }

    pub fn sublink_speed_attributes(&self) -> impl Iterator<Item = u32> {
        let ptr = unsafe { self.inner.as_ref() };
        let ptr = ptr::addr_of!(ptr.bmSublinkSpeedAttr);
        SublinkSpeedAttrs {
            inner: ptr as *const u32,
            lt: PhantomData,
        }
    }
}

pub struct SublinkSpeedAttrs<'a> {
    inner: *const u32,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for SublinkSpeedAttrs<'a> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.is_null() {
            return None;
        }
        let item = unsafe { self.inner.read_unaligned() };
        self.inner = unsafe { self.inner.add(1) };
        Some(item.clone())
    }
}

pub struct DeviceCapabilityContainerId<'a> {
    inner: NonNull<IOUSBDeviceCapabilityContainerID>,
    lt: PhantomData<&'a ()>,
}

impl DeviceCapabilityContainerId<'_> {
    fn new(ptr: *const IOUSBDeviceCapabilityContainerID) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBDeviceCapabilityContainerID)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn device_capability_type(&self) -> DeviceCapabilityType {
        unsafe { self.inner.as_ref().bDevCapabilityType }.into()
    }

    pub fn reserved_id(&self) -> u8 {
        unsafe { self.inner.as_ref().bReservedID }
    }

    pub fn container_id(&self) -> &[u8; 16] {
        unsafe { &self.inner.as_ref().containerID }
    }
}

pub struct PlatformCapabilityDescriptor<'a> {
    inner: NonNull<IOUSBPlatformCapabilityDescriptor>,
    lt: PhantomData<&'a ()>,
}

impl PlatformCapabilityDescriptor<'_> {
    fn new(ptr: *const IOUSBPlatformCapabilityDescriptor) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBPlatformCapabilityDescriptor)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn device_capability_type(&self) -> DeviceCapabilityType {
        unsafe { self.inner.as_ref().bDevCapabilityType }.into()
    }

    pub fn platform_capability_uuid(&self) -> &[u8; 16] {
        unsafe { &self.inner.as_ref().PlatformCapabilityUUID }
    }
}

pub struct DeviceCapabilityBillboard<'a> {
    inner: NonNull<IOUSBDeviceCapabilityBillboard>,
    lt: PhantomData<&'a ()>,
}

impl DeviceCapabilityBillboard<'_> {
    fn new(ptr: *const IOUSBDeviceCapabilityBillboard) -> Option<Self> {
        let inner = NonNull::new(ptr as *mut IOUSBDeviceCapabilityBillboard)?;
        Some(Self {
            inner,
            lt: PhantomData,
        })
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn device_capability_type(&self) -> DeviceCapabilityType {
        unsafe { self.inner.as_ref().bDevCapabilityType }.into()
    }

    ///index of string descriptor providing a URL for detailed information about the product and
    ///supported modes
    pub fn additional_info_url(&self) -> u8 {
        unsafe { self.inner.as_ref().iAdditionalInfoURL }
    }

    pub fn alternate_modes_count(&self) -> u8 {
        unsafe { self.inner.as_ref().bNumberOfAlternateModes }
    }

    pub fn preferred_alternate_mode(&self) -> u8 {
        unsafe { self.inner.as_ref().bPreferredAlternateMode }
    }

    pub fn connection_power(&self) -> u16 {
        unsafe { self.inner.as_ref().vCONNPower }
    }

    pub fn configured(&self) -> &[u8; 32] {
        unsafe { &self.inner.as_ref().bmConfigured }
    }

    pub fn bcd_version(&self) -> u16 {
        unsafe { self.inner.as_ref().bcdVersion }
    }

    pub fn additional_failure_info(&self) -> u8 {
        unsafe { self.inner.as_ref().bAdditionalFailureInfo }
    }

    pub fn alt_configurations(
        &self,
    ) -> impl Iterator<Item = DeviceCapabilityBillboardAltConfiguration<'_>> {
        let configs = unsafe { &self.inner.as_ref().pAltConfigurations };
        DeviceCapabilityBillboardAltConfigurations {
            inner: configs.as_ptr(),
            lt: PhantomData,
        }
    }
}

pub struct DeviceCapabilityBillboardAltConfigurations<'a> {
    inner: *const IOUSBDeviceCapabilityBillboardAltConfig,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for DeviceCapabilityBillboardAltConfigurations<'a> {
    type Item = DeviceCapabilityBillboardAltConfiguration<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = DeviceCapabilityBillboardAltConfiguration::new(self.inner)?;
        self.inner = unsafe { self.inner.add(1) };
        Some(next)
    }
}

pub struct DeviceCapabilityBillboardAltConfiguration<'a> {
    inner: NonNull<IOUSBDeviceCapabilityBillboardAltConfig>,
    lt: PhantomData<&'a ()>,
}

impl DeviceCapabilityBillboardAltConfiguration<'_> {
    fn new(ptr: *const IOUSBDeviceCapabilityBillboardAltConfig) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBDeviceCapabilityBillboardAltConfig)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }

    pub fn svid(&self) -> u16 {
        unsafe { self.inner.as_ref().wSVID }
    }

    pub fn altenate_mode(&self) -> u8 {
        unsafe { self.inner.as_ref().bAltenateMode }
    }

    /// index for alternate mode settings
    pub fn alternate_mode_setting(&self) -> u8 {
        unsafe { self.inner.as_ref().iAlternateModeString }
    }
}

pub struct CapabilityDescriptors<'a> {
    inner: *const IOUSBBOSDescriptor,
    lt: PhantomData<&'a ()>,
}

pub struct Capabilities<'a> {
    bos_descriptor: *const IOUSBBOSDescriptor,
    current_descriptor: *const IOUSBDeviceCapabilityDescriptorHeader,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for Capabilities<'a> {
    type Item = Capability<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextCapabilityDescriptor(self.bos_descriptor, self.current_descriptor)
        };

        if next.is_null() {
            return None;
        }

        self.current_descriptor = next;
        Some(Capability::new(next))
    }
}

pub struct TypedCapabilities<'a> {
    capability_type: u8,
    bos_descriptor: *const IOUSBBOSDescriptor,
    current_descriptor: *const IOUSBDeviceCapabilityDescriptorHeader,
    lt: PhantomData<&'a ()>,
}

impl<'a> Iterator for TypedCapabilities<'a> {
    type Item = Capability<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let next = unsafe {
            IOUSBGetNextCapabilityDescriptorWithType(
                self.bos_descriptor,
                self.current_descriptor,
                self.capability_type,
            )
        };

        if next.is_null() {
            return None;
        }

        self.current_descriptor = next;
        Some(Capability::new(next))
    }
}

pub struct Capability<'a> {
    inner: NonNull<IOUSBDeviceCapabilityDescriptorHeader>,
    lt: PhantomData<&'a IOUSBDeviceCapabilityDescriptorHeader>,
}

impl Capability<'_> {
    fn new(ptr: *const IOUSBDeviceCapabilityDescriptorHeader) -> Self {
        let ptr =
            unsafe { NonNull::new_unchecked(ptr as *mut IOUSBDeviceCapabilityDescriptorHeader) };
        Self {
            inner: ptr,
            lt: PhantomData,
        }
    }

    pub fn length(&self) -> u8 {
        unsafe { self.inner.as_ref().bLength }
    }

    pub fn descriptor_type(&self) -> DescriptorType {
        unsafe { self.inner.as_ref().bDescriptorType }.into()
    }

    pub fn device_capability_type(&self) -> DeviceCapabilityType {
        unsafe { self.inner.as_ref().bDevCapabilityType }.into()
    }
}

impl<'a> Iterator for CapabilityDescriptors<'a> {
    type Item = CapabilityDescriptor<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let ptr = unsafe { self.inner.add(1) };
        let next = CapabilityDescriptor::new(ptr)?;
        self.inner = ptr;
        Some(next)
    }
}

impl Drop for CapabilityDescriptor<'_> {
    fn drop(&mut self) {}
}

pub struct EndpointDescriptor<'a> {
    inner: NonNull<IOUSBEndpointDescriptor>,
    lt: PhantomData<&'a IOUSBEndpointDescriptor>,
}

impl EndpointDescriptor<'_> {
    fn new(ptr: *const IOUSBEndpointDescriptor) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBEndpointDescriptor)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }
    const SIZE: u8 = 7;

    pub fn size(&self) -> u8 {
        Self::SIZE
    }

    pub fn descriptor_type(&self) -> u8 {
        unsafe { self.inner.as_ref().bDescriptorType }
    }

    pub fn endpoint_address(&self) -> u8 {
        unsafe { self.inner.as_ref().bEndpointAddress }
    }

    pub fn interval(&self) -> u8 {
        unsafe { self.inner.as_ref().bInterval }
    }

    pub fn attributes(&self) -> u8 {
        unsafe { self.inner.as_ref().bmAttributes }
    }

    pub fn max_packet_size(&self) -> u16 {
        unsafe { self.inner.as_ref().wMaxPacketSize }
    }

    pub fn synchronization_type(&self) -> SynchronizationType {
        unsafe { IOUSBGetEndpointType(self.inner.as_ref()) }.into()
    }

    pub fn endpoint_direction(&self) -> EndpointDirection {
        unsafe { IOUSBGetEndpointDirection(self.inner.as_ref()) }.into()
    }

    pub fn endpoint_number(&self) -> u8 {
        unsafe { IOUSBGetEndpointNumber(self.inner.as_ref()) }
    }

    pub fn max_packet_size_with_device_speed(&self, usb_device_speed: u32) -> u16 {
        unsafe { IOUSBGetEndpointMaxPacketSize(usb_device_speed, self.inner.as_ref()) }
    }

    pub fn burst_size(
        &self,
        usb_device_speed: u32,
        super_speed_companion: &SuperSpeedCompanionDescriptor<'_>,
        super_speed_plus_companion: &SuperSpeedPlusCompanionDescriptor<'_>,
    ) -> u32 {
        unsafe {
            IOUSBGetEndpointBurstSize(
                usb_device_speed,
                self.inner.as_ref(),
                super_speed_companion.inner.as_ref(),
                super_speed_plus_companion.inner.as_ref(),
            )
        }
    }

    pub fn multiplier(
        &self,
        usb_device_speed: u32,
        super_speed_companion: &SuperSpeedCompanionDescriptor<'_>,
        super_speed_plus_companion: &SuperSpeedPlusCompanionDescriptor<'_>,
    ) -> u8 {
        unsafe {
            IOUSBGetEndpointMult(
                usb_device_speed,
                self.inner.as_ref(),
                super_speed_companion.inner.as_ref(),
                super_speed_plus_companion.inner.as_ref(),
            )
        }
    }

    pub fn interval_encoded_microframes(&self, usb_device_speed: u32) -> u32 {
        unsafe { IOUSBGetEndpointIntervalEncodedMicroframes(usb_device_speed, self.inner.as_ref()) }
    }

    pub fn interval_microframes(&self, usb_device_speed: u32) -> u32 {
        unsafe { IOUSBGetEndpointIntervalMicroframes(usb_device_speed, self.inner.as_ref()) }
    }

    pub fn interval_frames(&self, usb_device_speed: u32) -> u32 {
        unsafe { IOUSBGetEndpointIntervalFrames(usb_device_speed, self.inner.as_ref()) }
    }

    pub fn max_streams_encoded(
        &self,
        usb_device_speed: u32,
        super_speed_companion: &SuperSpeedCompanionDescriptor<'_>,
    ) -> u32 {
        unsafe {
            IOUSBGetEndpointMaxStreamsEncoded(
                usb_device_speed,
                self.inner.as_ref(),
                super_speed_companion.inner.as_ref(),
            )
        }
    }

    pub fn max_streams(
        &self,
        usb_device_speed: u32,
        super_speed_companion: &SuperSpeedCompanionDescriptor<'_>,
    ) -> u32 {
        unsafe {
            IOUSBGetEndpointMaxStreams(
                usb_device_speed,
                self.inner.as_ref(),
                super_speed_companion.inner.as_ref(),
            )
        }
    }
}

pub struct UsbHostObject<'a> {
    inner: NonNull<IOUSBHostObject>,
    lt: PhantomData<&'a ()>,
}

impl Drop for UsbHostObject<'_> {
    fn drop(&mut self) {
        unsafe { self.inner.as_ref().destroy() }
    }
}

pub struct DescriptorOptions {
    descriptor_type: DescriptorType,
    length: u64,
    language_options: Option<LanguageOptions>,
}

impl DescriptorOptions {
    pub fn new(
        descriptor_type: DescriptorType,
        length: u64,
        language_options: Option<LanguageOptions>,
    ) -> Self {
        Self {
            descriptor_type,
            length,
            language_options,
        }
    }
}

pub struct LanguageOptions {
    index: u64,
    language_id: u64,
    request_options: Option<RequestOptions>,
}

impl LanguageOptions {
    pub fn new(index: u64, language_id: u64, request_options: Option<RequestOptions>) -> Self {
        Self {
            index,
            language_id,
            request_options,
        }
    }
}

pub struct RequestOptions {
    request_type: DeviceRequestTypeValue,
    request_recipient: DeviceRequestRecipientValue,
}

impl RequestOptions {
    pub fn new(
        request_type: DeviceRequestTypeValue,
        request_recipient: DeviceRequestRecipientValue,
    ) -> Self {
        Self {
            request_type,
            request_recipient,
        }
    }
}

impl UsbHostObject<'_> {
    pub fn send_device_request_with_data(
        &self,
        request: DeviceRequest,
        data: &mut [u8],
    ) -> Result<u64, UsbError> {
        let data = MutData::with_data(data).raw();
        let mut err = NSErr::new();
        let mut transferred = 0;
        if !unsafe {
            self.inner
                .as_ref()
                .sendDeviceRequest_data_bytesTransferred_completionTimeout_error_(
                    request.into(),
                    data,
                    &mut transferred,
                    0.0,
                    &mut *err,
                )
        } {
            Err(err.into())
        } else {
            Ok(transferred)
        }
    }

    pub fn send_device_request(&self, request: DeviceRequest) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .sendDeviceRequest_error_(request.into(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub async fn enqueue_device_request_with_data(
        &self,
        request: DeviceRequest,
        data: &[u8],
    ) -> Result<(), UsbError> {
        let handler = AsyncDataHandler::new(self.inner, data, |dev, data, cb| {
            let cb = unsafe { downcast_tait(cb) };

            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueDeviceRequest_data_completionTimeout_error_completionHandler_(
                    request.into(),
                    data,
                    0.0,
                    &mut *err,
                    cb,
                )
            } {
                Some(err.into())
            } else {
                None
            }
        });

        handler.await
    }

    pub async fn enqueue_device_request(&self, request: DeviceRequest) -> Result<(), UsbError> {
        let handler = AsyncHandler::new(self.inner, |dev, cb| {
            let cb = unsafe { downcast_tait(cb) };
            let mut err = NSErr::new();
            if !unsafe {
                dev.enqueueDeviceRequest_error_completionHandler_(request.into(), &mut *err, cb)
            } {
                Some(err.into())
            } else {
                None
            }
        });
        handler.await
    }

    pub fn abort_device_requests(&self, option: AbortOption) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .as_ref()
                .abortDeviceRequestsWithOption_error_(option.into(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn descriptor(&self, options: DescriptorOptions) -> Result<DescriptorHeader<'_>, UsbError> {
        let mut err = NSErr::new();
        let DescriptorOptions {
            descriptor_type,
            mut length,
            language_options,
        } = options;
        let descriptor_type: u8 = descriptor_type.into();
        let desc = unsafe {
            match language_options {
                Some(LanguageOptions {
                    index,
                    language_id,
                    request_options:
                        Some(RequestOptions {
                            request_type,
                            request_recipient,
                        }),
                }) => self
                    .inner
                    .as_ref()
                    .descriptorWithType_length_index_languageID_requestType_requestRecipient_error_(
                        descriptor_type as u32,
                        &mut length,
                        index,
                        language_id,
                        request_type.into(),
                        request_recipient.into(),
                        &mut *err,
                    ),
                Some(LanguageOptions {
                    index,
                    language_id,
                    request_options: None,
                }) => self
                    .inner
                    .as_ref()
                    .descriptorWithType_length_index_languageID_error_(
                        descriptor_type as u32,
                        &mut length,
                        index,
                        language_id,
                        &mut *err,
                    ),
                None => self.inner.as_ref().descriptorWithType_length_error_(
                    descriptor_type as u32,
                    &mut length,
                    &mut *err,
                ),
            }
        };

        if err.is_err() {
            Err(err.into())
        } else {
            Ok(DescriptorHeader::new(desc))
        }
    }

    pub fn string_descriptor(
        &self,
        index: u64,
        language_id: Option<u64>,
    ) -> Result<NSString, UsbError> {
        let mut err = NSErr::new();
        let desc = unsafe {
            match language_id {
                Some(id) => self
                    .inner
                    .as_ref()
                    .stringWithIndex_languageID_error_(index, id, &mut *err),
                None => self.inner.as_ref().stringWithIndex_error_(index, &mut *err),
            }
        };

        if err.is_err() {
            Err(err.into())
        } else {
            Ok(desc)
        }
    }

    pub fn configuration_descriptors(&self) -> impl Iterator<Item = ConfigurationDescriptor<'_>> {
        let count = self.device_descriptor().unwrap().configuration_count();
        let idx = 0;
        ConfigurationDescriptors {
            configuration_count: count,
            idx,
            dev: unsafe { self.inner.as_ref() },
        }
    }

    pub fn configuration_descriptor_with_value(
        &self,
        val: u64,
    ) -> Result<ConfigurationDescriptor<'_>, UsbError> {
        let mut err = NSErr::new();
        let desc = unsafe {
            self.inner
                .as_ref()
                .configurationDescriptorWithConfigurationValue_error_(val, &mut *err)
        };

        if err.is_err() {
            Err(err.into())
        } else {
            Ok(ConfigurationDescriptor::new(desc).unwrap())
        }
    }

    //returns the current frame number, but also updates the host time aligned with the time which
    //the frame number was last updated
    pub fn frame_number(&self, time: &mut HostTime) -> u64 {
        unsafe { self.inner.as_ref().frameNumberWithTime_(&mut time.inner) }
    }

    pub fn io_data(&self, capacity: u64) -> Result<NSMutableData, UsbError> {
        let mut err = NSErr::new();
        let data = unsafe {
            self.inner
                .as_ref()
                .ioDataWithCapacity_error_(capacity, &mut *err)
        };
        if err.is_err() {
            Err(err.into())
        } else {
            Ok(data)
        }
    }

    pub fn queue(&self) -> Queue {
        Queue::new(unsafe { self.inner.as_ref().queue() })
    }

    pub fn device_descriptor(&self) -> Option<DeviceDescriptor> {
        let ptr = unsafe { self.inner.as_ref().deviceDescriptor() };
        DeviceDescriptor::new(ptr)
    }

    pub fn capability_descriptors(&self) -> impl Iterator<Item = CapabilityDescriptor<'_>> {
        let ptr = unsafe { self.inner.as_ref().capabilityDescriptors() };
        CapabilityDescriptors {
            inner: ptr,
            lt: PhantomData,
        }
    }

    pub fn device_address(&self) -> u64 {
        unsafe { self.inner.as_ref().deviceAddress() }
    }
}

pub enum AbortOption {
    Asynchronous = 0,
    Synchronous = 1,
}

impl From<AbortOption> for IOUSBHostAbortOption {
    fn from(option: AbortOption) -> IOUSBHostAbortOption {
        use AbortOption as AO;
        match option {
            AO::Asynchronous => 0,
            AO::Synchronous => 1,
        }
    }
}

pub struct ConfigurationDescriptors<'a> {
    dev: &'a IOUSBHostObject,
    idx: u8,
    configuration_count: u8,
}

impl<'a> Iterator for ConfigurationDescriptors<'a> {
    type Item = ConfigurationDescriptor<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.configuration_count - 1 {
            return None;
        }

        let mut err = NSErr::new();
        let ptr = unsafe {
            self.dev
                .configurationDescriptorWithIndex_error_(self.idx as u64, &mut *err)
        };

        if err.is_err() {
            let err: UsbError = err.into();
            println!("err while enumerating configuration descriptors: {:?}", err);
            return None;
        }

        self.idx += 1;
        ConfigurationDescriptor::new(ptr)
    }
}

pub struct EndpointStateMachine {
    inner: IOUSBHostCIEndpointStateMachine,
}

impl EndpointStateMachine {
    pub fn inspect_command(&self, command: &Message<'_>) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .inspectCommand_error_(command.inner.as_ref(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn respond(&self, command: &Message<'_>, status: MessageStatus) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner.respondToCommand_status_error_(
                command.inner.as_ref(),
                status.into(),
                &mut *err,
            )
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn process_doorbell(&self, doorbell: u32) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe { self.inner.processDoorbell_error_(doorbell, &mut *err) } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn enqueue_transfer_completion_for_message(
        &self,
        message: &Message<'_>,
        status: MessageStatus,
        transfer_length: u64,
    ) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .enqueueTransferCompletionForMessage_status_transferLength_error_(
                    message.inner.as_ref(),
                    status.into(),
                    transfer_length,
                    &mut *err,
                )
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn endpoint_state(&self) -> EndpointState {
        unsafe { self.inner.endpointState() }.into()
    }

    pub fn device_address(&self) -> u64 {
        unsafe { self.inner.deviceAddress() }
    }

    pub fn endpoint_address(&self) -> u64 {
        unsafe { self.inner.endpointAddress() }
    }

    pub fn current_transfer_message(&mut self) -> Option<Message<'_>> {
        Message::new(unsafe { self.inner.currentTransferMessage() })
    }

    pub fn controller_interface(&self) -> ControllerInterface {
        ControllerInterface::new(unsafe { self.inner.controllerInterface() })
    }
}

pub struct Message<'a> {
    inner: NonNull<IOUSBHostCIMessage>,
    lt: PhantomData<&'a ()>,
}

impl Message<'_> {
    fn new(ptr: *const IOUSBHostCIMessage) -> Option<Self> {
        let ptr = NonNull::new(ptr as *mut IOUSBHostCIMessage)?;
        Some(Self {
            inner: ptr,
            lt: PhantomData,
        })
    }
    pub fn control(&self) -> u32 {
        unsafe { self.inner.as_ref().control }
    }

    pub fn data_0(&self) -> u32 {
        unsafe { self.inner.as_ref().data0 }
    }

    pub fn data_1(&self) -> u64 {
        unsafe { self.inner.as_ref().data1 }
    }

    pub fn data(&self) -> (u32, u64) {
        let msg = unsafe { self.inner.as_ref() };
        (msg.data0, msg.data1)
    }
}

pub struct ControllerInterface {
    inner: IOUSBHostControllerInterface,
}

impl ControllerInterface {
    fn new(inner: IOUSBHostControllerInterface) -> Self {
        Self { inner }
    }

    pub fn enqueue_interrupts(
        &self,
        msg: &Message<'_>,
        expedited: Option<bool>,
        count: Option<u64>,
    ) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        let is_err = unsafe {
            match (expedited, count) {
                (Some(expd), Some(count)) => self.inner.enqueueInterrupts_count_expedite_error_(
                    msg.inner.as_ref(),
                    count,
                    expd,
                    &mut *err,
                ),
                (Some(expd), None) => {
                    self.inner
                        .enqueueInterrupt_expedite_error_(msg.inner.as_ref(), expd, &mut *err)
                }
                (None, Some(count)) => {
                    self.inner
                        .enqueueInterrupts_count_error_(msg.inner.as_ref(), count, &mut *err)
                }
                (None, None) => self
                    .inner
                    .enqueueInterrupt_error_(msg.inner.as_ref(), &mut *err),
            }
        };
        if is_err {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn message_description(&self, msg: &Message<'_>) -> NSString {
        unsafe { self.inner.descriptionForMessage_(msg.inner.as_ref()) }
    }

    pub fn port_state_machine_for_command(
        &self,
        cmd: &Message<'_>,
    ) -> Result<PortStateMachine, UsbError> {
        let mut err = NSErr::new();
        let res = unsafe {
            self.inner
                .getPortStateMachineForCommand_error_(cmd.inner.as_ref(), &mut *err)
        };
        if err.is_err() {
            Err(err.into())
        } else {
            Ok(PortStateMachine::new(res))
        }
    }

    pub fn port_state_machine_for_port(&self, port: u64) -> Result<PortStateMachine, UsbError> {
        let mut err = NSErr::new();
        let res = unsafe {
            self.inner
                .getPortStateMachineForPort_error_(port, &mut *err)
        };
        if err.is_err() {
            Err(err.into())
        } else {
            Ok(PortStateMachine::new(res))
        }
    }

    pub fn port_capabilities(&self, port: u64) -> Option<Message<'_>> {
        Message::new(unsafe { self.inner.capabilitiesForPort_(port) })
    }

    pub fn queue(&self) -> Queue {
        Queue::new(unsafe { self.inner.queue() })
    }

    pub fn interupt_rate_hz(&self) -> u64 {
        unsafe { self.inner.interruptRateHz() }
    }

    pub fn set_interrupt_rate_hz(&self, rate: u64) {
        unsafe { self.inner.setInterruptRateHz_(rate) }
    }

    pub fn controller_state_machine(&self) -> ControllerStateMachine {
        ControllerStateMachine::new(unsafe { self.inner.controllerStateMachine() })
    }

    pub fn capabilities(&self) -> Option<Message<'_>> {
        Message::new(unsafe { self.inner.capabilities() })
    }

    pub fn uuid(&self) -> NSUUID {
        unsafe { self.inner.uuid() }
    }
}

pub struct ControllerStateMachine {
    inner: IOUSBHostCIControllerStateMachine,
}

impl ControllerStateMachine {
    fn new(inner: IOUSBHostCIControllerStateMachine) -> Self {
        Self { inner }
    }

    pub fn inspect_command(&self, cmd: &Message<'_>) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .inspectCommand_error_(cmd.inner.as_ref(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn respond(
        &self,
        cmd: &Message<'_>,
        status: MessageStatus,
        frame_timestamp: Option<(u64, u64)>,
    ) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        let is_err = unsafe {
            match frame_timestamp {
                Some((frame, timestamp)) => {
                    self.inner.respondToCommand_status_frame_timestamp_error_(
                        cmd.inner.as_ref(),
                        status.into(),
                        frame,
                        timestamp,
                        &mut *err,
                    )
                }
                None => self.inner.respondToCommand_status_error_(
                    cmd.inner.as_ref(),
                    status.into(),
                    &mut *err,
                ),
            }
        };
        if is_err {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn enqueue_updated(&self, frame: u64, timestamp: u64) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .enqueueUpdatedFrame_timestamp_error_(frame, timestamp, &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn controller_state(&self) -> ControllerState {
        unsafe { self.inner.controllerState() }.into()
    }

    pub fn controller_interface(&self) -> ControllerInterface {
        ControllerInterface::new(unsafe { self.inner.controllerInterface() })
    }
}

pub struct PortStateMachine {
    inner: IOUSBHostCIPortStateMachine,
}

impl PortStateMachine {
    fn new(inner: IOUSBHostCIPortStateMachine) -> Self {
        Self { inner }
    }

    pub fn inspect_command(&self, cmd: &Message<'_>) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .inspectCommand_error_(cmd.inner.as_ref(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn respond(&self, cmd: &Message<'_>, status: MessageStatus) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .respondToCommand_status_error_(cmd.inner.as_ref(), status.into(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn update_link_state(
        &self,
        link_state: LinkState,
        speed: DeviceSpeed,
        inhibit_link_state_change: bool,
    ) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .updateLinkState_speed_inhibitLinkStateChange_error_(
                    link_state.into(),
                    speed.into(),
                    inhibit_link_state_change,
                    &mut *err,
                )
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn port_state(&self) -> PortState {
        unsafe { self.inner.portState() }.into()
    }

    pub fn port_status(&self) -> PortStatus {
        unsafe { self.inner.portStatus() }.into()
    }

    pub fn controller_interface(&self) -> ControllerInterface {
        ControllerInterface::new(unsafe { self.inner.controllerInterface() })
    }

    pub fn powered(&self) -> bool {
        unsafe { self.inner.powered() }
    }

    pub fn set_powered(&self, powered: bool) {
        unsafe { self.inner.setPowered_(powered) }
    }

    pub fn connected(&self) -> bool {
        unsafe { self.inner.connected() }
    }

    pub fn set_connected(&self, connected: bool) {
        unsafe { self.inner.setConnected_(connected) }
    }

    pub fn overcurrent(&self) -> bool {
        unsafe { self.inner.overcurrent() }
    }

    pub fn set_overcurrent(&self, overcurrent: bool) {
        unsafe { self.inner.setOvercurrent_(overcurrent) }
    }

    pub fn link_state(&self) -> LinkState {
        unsafe { self.inner.linkState() }.into()
    }

    pub fn speed(&self) -> DeviceSpeed {
        unsafe { self.inner.speed() }.into()
    }
}

impl Drop for ControllerInterface {
    fn drop(&mut self) {
        unsafe { self.inner.destroy() }
    }
}

pub struct DeviceStateMachine {
    inner: IOUSBHostCIDeviceStateMachine,
}

impl DeviceStateMachine {
    pub fn inspect_command(&self, cmd: &Message<'_>) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        if !unsafe {
            self.inner
                .inspectCommand_error_(cmd.inner.as_ref(), &mut *err)
        } {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn respond(
        &self,
        cmd: &Message,
        status: MessageStatus,
        device_address: Option<u64>,
    ) -> Result<(), UsbError> {
        let mut err = NSErr::new();
        let is_err = unsafe {
            match device_address {
                Some(addr) => self.inner.respondToCommand_status_deviceAddress_error_(
                    cmd.inner.as_ref(),
                    status.into(),
                    addr,
                    &mut *err,
                ),
                None => self.inner.respondToCommand_status_error_(
                    cmd.inner.as_ref(),
                    status.into(),
                    &mut *err,
                ),
            }
        };
        if is_err {
            Err(err.into())
        } else {
            Ok(())
        }
    }

    pub fn device_state(&self) -> DeviceState {
        unsafe { self.inner.deviceState() }.into()
    }

    pub fn complete_route(&self) -> u64 {
        unsafe { self.inner.completeRoute() }
    }

    pub fn device_address(&self) -> u64 {
        unsafe { self.inner.deviceAddress() }
    }

    pub fn controller_interface(&self) -> ControllerInterface {
        ControllerInterface::new(unsafe { self.inner.controllerInterface() })
    }
}

pub struct IoService {
    inner: io_service_t,
}

impl IoService {
    pub fn authorize(&self, options: u32) -> Result<(), i32> {
        let res = unsafe { IOServiceAuthorize(self.inner, options) };
        if res != 0 {
            Err(res)
        } else {
            Ok(())
        }
    }

    fn from_raw(raw: io_service_t) -> Self {
        Self { inner: raw }
    }
}

pub struct MutData {
    inner: NSMutableData,
}

impl MutData {
    pub fn with_data(data: &[u8]) -> Self {
        let mut_data = NSMutableData::alloc();
        let len = data.len() as u64;
        unsafe {
            mut_data.initWithCapacity_(len);
            mut_data.appendBytes_length_(data.as_ptr() as *const c_void, len);
        }
        Self { inner: mut_data }
    }

    fn raw(self) -> NSMutableData {
        self.inner
    }
}

type Callback = impl FnOnce();

fn gen_callback(waker: Waker, finished: *const std::sync::Mutex<bool>) -> Callback {
    move || {
        let finished = &mut *unsafe { finished.as_ref().unwrap().lock().unwrap() };
        *finished = true;
        waker.wake()
    }
}

//NOTE: if we could get rid of either of these mutexes that would be great

///used for handling async events which sends data
struct AsyncDataHandler<'a, F: Fn(&'a T, NSMutableData, *mut Callback) -> Option<UsbError>, T> {
    handler: std::sync::Mutex<*mut Callback>,
    dev: &'a T,
    data: NSMutableData,
    cb_handler: F,
    finished: std::sync::Mutex<bool>,
}

impl<'a, T, F: Fn(&'a T, NSMutableData, *mut Callback) -> Option<UsbError>>
    AsyncDataHandler<'a, F, T>
{
    fn new(dev: NonNull<T>, data: &[u8], cb_handler: F) -> Self {
        let data = MutData::with_data(data).raw();
        let dev = unsafe { dev.as_ref() };
        Self {
            dev,
            cb_handler,
            data,
            handler: std::sync::Mutex::new(ptr::null_mut()),
            finished: std::sync::Mutex::new(false),
        }
    }
}

impl<'a, T, F: Fn(&'a T, NSMutableData, *mut Callback) -> Option<UsbError>> Future
    for AsyncDataHandler<'a, F, T>
{
    type Output = Result<(), UsbError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.finished.lock().as_deref() {
            Ok(true) => Poll::Ready(Ok(())),
            Ok(false) => {
                let boxed = Box::new(gen_callback(cx.waker().clone(), &self.finished));
                let handler = Box::into_raw(boxed);
                let h = &mut *self.handler.lock().unwrap();
                *h = handler;
                if let Some(err) = (self.cb_handler)(self.dev, self.data, handler) {
                    Poll::Ready(Err(err))
                } else {
                    Poll::Pending
                }
            }
            _ => {
                todo!()
            }
        }
    }
}

///used for handling async events which does not send data
struct AsyncHandler<'a, F: Fn(&'a T, *mut Callback) -> Option<UsbError>, T> {
    handler: std::sync::Mutex<*mut Callback>,
    dev: &'a T,
    cb_handler: F,
    finished: std::sync::Mutex<bool>,
}

impl<'a, T, F: Fn(&'a T, *mut Callback) -> Option<UsbError>> AsyncHandler<'a, F, T> {
    fn new(dev: NonNull<T>, cb_handler: F) -> Self {
        let dev = unsafe { dev.as_ref() };
        Self {
            dev,
            cb_handler,
            handler: std::sync::Mutex::new(ptr::null_mut()),
            finished: std::sync::Mutex::new(false),
        }
    }
}

impl<'a, T, F: Fn(&'a T, *mut Callback) -> Option<UsbError>> Future for AsyncHandler<'a, F, T> {
    type Output = Result<(), UsbError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.finished.lock().as_deref() {
            Ok(true) => Poll::Ready(Ok(())),
            Ok(false) => {
                let boxed = Box::new(gen_callback(cx.waker().clone(), &self.finished));
                let handler = Box::into_raw(boxed);
                let h = &mut *self.handler.lock().unwrap();
                *h = handler;
                if let Some(err) = (self.cb_handler)(self.dev, handler) {
                    Poll::Ready(Err(err))
                } else {
                    Poll::Pending
                }
            }
            _ => {
                todo!()
            }
        }
    }
}

/// SAFETY: i have no clue if this works.
/// this might be breaking
unsafe fn downcast_tait(tait: *mut Callback) -> *mut c_void {
    tait as *mut dyn FnOnce() as *mut c_void
}

#[repr(transparent)]
pub struct IsochronousFrame {
    inner: IOUSBHostIsochronousFrame,
}

pub enum Status {
    Ok,
    Err(UsbError),
}

impl From<Status> for i32 {
    fn from(status: Status) -> i32 {
        match status {
            Status::Ok => 0,
            Status::Err(err) => err.into(),
        }
    }
}

impl IsochronousFrame {
    pub fn new(
        status: Status,
        request_count: u32,
        complete_count: u32,
        timestamp: HostTime,
    ) -> Self {
        let inner = IOUSBHostIsochronousFrame {
            status: status.into(),
            requestCount: request_count,
            completeCount: complete_count,
            reserved: 0,
            timeStamp: timestamp.inner,
        };
        Self { inner }
    }
}

#[repr(transparent)]
pub struct IsochronousTransaction {
    inner: IOUSBHostIsochronousTransaction,
}

impl IsochronousTransaction {
    pub fn new(
        status: Status,
        request_count: u32,
        offset: u32,
        complete_count: u32,
        timestamp: HostTime,
        options: IsochronousTransactionOptions,
    ) -> Self {
        let inner = IOUSBHostIsochronousTransaction {
            status: status.into(),
            requestCount: request_count,
            offset,
            completeCount: complete_count,
            timeStamp: timestamp.inner,
            options: options.into(),
        };
        Self { inner }
    }
}

#[derive(Clone, Copy)]
pub enum IsochronousTransactionOptions {
    None = 0,
    Wrap = 1,
}

impl From<IsochronousTransactionOptions> for IOUSBHostIsochronousTransferOptions {
    fn from(options: IsochronousTransactionOptions) -> IOUSBHostIsochronousTransferOptions {
        use IsochronousTransactionOptions as ITO;
        match options {
            ITO::None => 0,
            ITO::Wrap => 1,
        }
    }
}

pub struct HostTime {
    inner: u64,
}

impl From<std::time::Instant> for HostTime {
    fn from(_instant: std::time::Instant) -> HostTime {
        todo!()
    }
}

pub enum Exception {
    Unknown = 0,
    InvalidCapabilities = 1,
    Terminated = 2,
    CommandReadCollision = 3,
    WriteFailed = 4,
    Timeout = 5,
    Failure = 6,
    InvalidInterrupt = 7,
    InterruptOverflow = 8,
    DoorbellReadCollision = 9,
    DoorbellOverflow = 10,
    ProtocolError = 11,
    FrameUpdateError = 12,
}

pub enum MessageType {
    ControllerCapabilities = 0,
    PortCapabilities = 1,
    PortEvent = 8,
    FrameNumberUpdate = 9,
    FrameTimestampUpdate = 10,
    ControllerPowerOn = 16,
    ControllerPowerOff = 17,
    ControllerStart = 18,
    ControllerPause = 19,
    ControllerFrameNumner = 20,
    PortPowerOn = 24,
    PortPowerOff = 25,
    PortResume = 26,
    PortSuspend = 27,
    PortReset = 28,
    PortDisable = 29,
    PortStatus = 30,
    DeviceCreate = 32,
    DeviceDestroy = 33,
    DeviceStart = 34,
    DevicePause = 35,
    DeviceUpdate = 36,
    EndpointCreate = 40,
    EndpointDesroy = 41,
    EndpointPause = 43,
    EndpointUpdate = 44,
    EndpointRest = 45,
    EndpointSetNextTransfer = 46,
    CommandMax = 55,
    SetupTransfer = 56,
    NormalTransfer = 57,
    StatusTransfer = 58,
    IsochronousTransfer = 59,
    Link = 60,
    TransferComplete = 61,
}

#[repr(u32)]
pub enum MessageStatus {
    Success = 1,
    Offline = 2,
    NotPermitted = 3,
    BadArgument = 4,
    Timeout = 5,
    NoResources = 6,
    EndpointStopped = 7,
    ProtocolError = 8,
    TransactionError = 9,
    OverrunError = 10,
    StallError = 11,
    MissedServiceError = 12,
    Error = 13,
    Other(u32),
}

impl From<MessageStatus> for u32 {
    fn from(status: MessageStatus) -> u32 {
        use MessageStatus as MS;
        match status {
            MS::Success => 1,
            MS::Offline => 2,
            MS::NotPermitted => 3,
            MS::BadArgument => 4,
            MS::Timeout => 5,
            MS::NoResources => 6,
            MS::EndpointStopped => 7,
            MS::ProtocolError => 8,
            MS::TransactionError => 9,
            MS::OverrunError => 10,
            MS::StallError => 11,
            MS::MissedServiceError => 12,
            MS::Error => 13,
            MS::Other(other) => other,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u32)]
pub enum DeviceSpeed {
    None = 0,
    Full = 1,
    Low = 2,
    High = 3,
    Super = 4,
    SuperPlus = 5,
    SuperPlusBy2 = 6,
    Other(u32),
}

impl From<u32> for DeviceSpeed {
    fn from(num: u32) -> DeviceSpeed {
        use DeviceSpeed as DS;
        match num {
            0 => DS::None,
            1 => DS::Full,
            2 => DS::Low,
            3 => DS::High,
            4 => DS::Super,
            5 => DS::SuperPlus,
            6 => DS::SuperPlusBy2,
            other => DS::Other(other),
        }
    }
}

impl From<DeviceSpeed> for u32 {
    fn from(speed: DeviceSpeed) -> u32 {
        use DeviceSpeed as DS;
        match speed {
            DS::None => 0,
            DS::Full => 1,
            DS::Low => 2,
            DS::High => 3,
            DS::Super => 4,
            DS::SuperPlus => 5,
            DS::SuperPlusBy2 => 6,
            DS::Other(other) => other,
        }
    }
}

#[repr(u32)]
pub enum LinkState {
    U0 = 0,
    U1 = 1,
    U2 = 2,
    U3 = 3,
    Disabled = 4,
    RxDetect = 5,
    Inactive = 6,
    Polling = 7,
    Recovery = 8,
    Reset = 9,
    Compliance = 10,
    Test = 11,
    Resume = 15,
    Other(u32),
}

impl From<u32> for LinkState {
    fn from(num: u32) -> LinkState {
        use LinkState as LS;
        match num {
            0 => LS::U0,
            1 => LS::U1,
            2 => LS::U2,
            3 => LS::U3,
            4 => LS::Disabled,
            5 => LS::RxDetect,
            6 => LS::Inactive,
            7 => LS::Polling,
            8 => LS::Recovery,
            9 => LS::Reset,
            10 => LS::Compliance,
            11 => LS::Test,
            15 => LS::Resume,
            other => LS::Other(other),
        }
    }
}

impl From<LinkState> for u32 {
    fn from(state: LinkState) -> u32 {
        use LinkState as LS;
        match state {
            LS::U0 => 0,
            LS::U1 => 1,
            LS::U2 => 2,
            LS::U3 => 3,
            LS::Disabled => 4,
            LS::RxDetect => 5,
            LS::Inactive => 6,
            LS::Polling => 7,
            LS::Recovery => 8,
            LS::Reset => 9,
            LS::Compliance => 10,
            LS::Test => 11,
            LS::Resume => 15,
            LS::Other(other) => other,
        }
    }
}

#[repr(u32)]
pub enum PortStatus {
    Powered = 1,
    Overcurrent = 2,
    Connected = 4,
    LinkState = 240,
    Speed = 1792,
    SpeedPhase = 8,
    OvercurrentChange = 131072,
    ConnectChange = 262144,
    LinkStateChange = 1048576,
    Other(u32),
}

impl From<u32> for PortStatus {
    fn from(num: u32) -> PortStatus {
        use PortStatus as PS;
        match num {
            1 => PS::Powered,
            2 => PS::Overcurrent,
            4 => PS::Connected,
            240 => PS::LinkState,
            1792 => PS::Speed,
            8 => PS::SpeedPhase,
            131072 => PS::OvercurrentChange,
            262144 => PS::ConnectChange,
            1048576 => PS::LinkStateChange,
            other => PS::Other(other),
        }
    }
}

pub enum Doorbell {
    DeviceAddress = 255,
    DeviceAddressPhase = 0,
    EndpointAddress = 65280,
    EndpointAddressPhase = 8,
    StreamId = 4294901760,
    StreamIDPhase = 16,
}

pub enum MessageCommand {
    ControlStatus = 3840,
    StatusPhase = 8,
    Data0RootPort = 15,
    RootPortPhase = 0,
    Data0DeviceAddress = 255,
    Data0EndpointAddress = 65280,
    Data0StreamId = 4294901760,
    Data0StreamIdPhase = 16,
}

#[repr(u32)]
pub enum ControllerState {
    Off = 0,
    Paused = 1,
    Active = 2,
    Other(u32),
}

impl From<u32> for ControllerState {
    fn from(num: u32) -> ControllerState {
        use ControllerState as CS;
        match num {
            0 => CS::Off,
            1 => CS::Paused,
            2 => CS::Active,
            other => CS::Other(other),
        }
    }
}

#[repr(u32)]
pub enum PortState {
    Off = 0,
    Powered = 1,
    Suspended = 2,
    Active = 3,
    Other(u32),
}

impl From<u32> for PortState {
    fn from(num: u32) -> PortState {
        use PortState as PS;
        match num {
            0 => PS::Off,
            1 => PS::Powered,
            2 => PS::Suspended,
            3 => PS::Active,
            other => PS::Other(other),
        }
    }
}

pub enum PortMessageEvent {
    Data0PortNumber = 15,
    Data0PortNumberPhase = 0,
}

pub enum PortStatusCommand {
    Powered = 1,
    Overcurrent = 2,
    Connected = 4,
    LinkState = 240,
    Speed = 1792,
    SpeedPhase = 8,
    OvercurrentChange = 131072,
    ConnectChange = 262144,
    LinkStateChange = 1048576,
    ChangeMask = 1441792,
}

#[repr(u32)]
pub enum DeviceState {
    Destroyed = 0,
    Paused = 1,
    Active = 2,
    Other(u32),
}

impl From<u32> for DeviceState {
    fn from(num: u32) -> DeviceState {
        use DeviceState as DS;
        match num {
            0 => DS::Destroyed,
            1 => DS::Paused,
            2 => DS::Active,
            other => DS::Other(other),
        }
    }
}

#[repr(u32)]
pub enum EndpointState {
    Destroyed = 0,
    Halted = 1,
    Paused = 2,
    Active = 3,
    Other(u32),
}

impl From<u32> for EndpointState {
    fn from(num: u32) -> EndpointState {
        use EndpointState as ES;
        match num {
            0 => ES::Destroyed,
            1 => ES::Halted,
            2 => ES::Paused,
            3 => ES::Active,
            other => ES::Other(other),
        }
    }
}

#[repr(u64)]
pub enum EndpointCreateCommand {
    Data1Descriptor = 18446744073709551615,
    Data1DescriptorPhase = 0,
}

#[repr(u64)]
pub enum EndpointUpdateCommand {
    Data1Descriptor = 18446744073709551615,
    Data1DescriptorPhase = 0,
}

pub enum EndpointResetCommand {
    Data1ClearState = 1,
}

#[repr(u64)]
pub enum EndpointSetNExtTransferCommand {
    Data1Address = 18446744073709551615,
    Data1AddressPhase = 0,
}

#[repr(u64)]
pub enum TransferCompletionMessage {
    Status = 3840,
    StatusPhase = 8,
    DeviceAddress = 16711680,
    DeviceAddressPhase = 16,
    EndpointAddress = 4278190080,
    EndpointAddressPhase = 24,
    Data0TransferLength = 268435455,
    Data0TransferLengthPhase = 0,
    Data1TransferStructure = 18446744073709551615,
}

pub enum EndpointDirection {
    Out = 0,
    In = 1,
    Unknown = 2,
}

impl From<u8> for EndpointDirection {
    fn from(num: u8) -> EndpointDirection {
        use EndpointDirection as ED;
        match num {
            0 => ED::Out,
            1 => ED::In,
            _ => ED::Unknown,
        }
    }
}

pub enum EndpointType {
    Control = 0,
    Isochronous = 1,
    Bulk = 2,
    Interrupt = 3,
}

#[repr(u8)]
pub enum SynchronizationType {
    None = 0,
    Asynchronous = 1,
    Adaptive = 2,
    Synchronous = 3,
    Other(u8),
}

impl From<u8> for SynchronizationType {
    fn from(num: u8) -> SynchronizationType {
        use SynchronizationType as ST;
        match num {
            0 => ST::None,
            1 => ST::Asynchronous,
            2 => ST::Adaptive,
            3 => ST::Synchronous,
            other => ST::Other(other),
        }
    }
}

#[repr(u8)]
pub enum DeviceCapabilityType {
    Wireless = 1,
    Usb2Extension = 2,
    SuperSpeed = 3,
    ContainerID = 4,
    Platform = 5,
    PowerDelivery = 6,
    BatteryInfo = 7,
    PdConsumerPort = 8,
    PdProviderPort = 9,
    SuperSpeedPlus = 10,
    PrecisionMeasurement = 11,
    WirelessExt = 12,
    Billboard = 13,
    BillboardAltMode = 15,
    Other(u8),
}

impl From<u8> for DeviceCapabilityType {
    fn from(num: u8) -> DeviceCapabilityType {
        use DeviceCapabilityType as DCT;
        match num {
            1 => DCT::Wireless,
            2 => DCT::Usb2Extension,
            3 => DCT::SuperSpeed,
            4 => DCT::ContainerID,
            5 => DCT::Platform,
            6 => DCT::PowerDelivery,
            7 => DCT::BatteryInfo,
            8 => DCT::PdConsumerPort,
            9 => DCT::PdProviderPort,
            10 => DCT::SuperSpeedPlus,
            11 => DCT::PrecisionMeasurement,
            12 => DCT::WirelessExt,
            13 => DCT::Billboard,
            15 => DCT::BillboardAltMode,
            other => DCT::Other(other),
        }
    }
}

pub enum DeviceRequestDirectionValue {
    Out = 0,
    In = 1,
}

#[repr(u32)]
pub enum DeviceRequestTypeValue {
    Standard = 0,
    Class = 1,
    Vendor = 2,
    Other(u32),
}

impl From<DeviceRequestTypeValue> for u32 {
    fn from(req: DeviceRequestTypeValue) -> u32 {
        use DeviceRequestTypeValue as DRTV;
        match req {
            DRTV::Standard => 0,
            DRTV::Class => 1,
            DRTV::Vendor => 2,
            DRTV::Other(other) => other,
        }
    }
}

pub enum DeviceRequestRecipientValue {
    Device = 0,
    Interface = 1,
    Endpoint = 2,
    Other = 3,
}

impl From<DeviceRequestRecipientValue> for u32 {
    fn from(val: DeviceRequestRecipientValue) -> u32 {
        use DeviceRequestRecipientValue as DRRV;
        match val {
            DRRV::Device => 0,
            DRRV::Interface => 1,
            DRRV::Endpoint => 2,
            DRRV::Other => 3,
        }
    }
}

#[repr(u8)]
pub enum DeviceRequestType {
    Size = 8,
    DirectionPhase = 7,
    DirectionOut = 0,
    DirectionIn = 128,
    TypePhase = 5,
    TypeClass = 32,
    TypeVendor = 64,
    RecipientInterface = 1,
    RecipientEndpoint = 2,
    RecipientOther = 3,
    Other(u8),
}

impl From<DeviceRequestType> for u8 {
    fn from(req_ty: DeviceRequestType) -> u8 {
        use DeviceRequestType as DRT;
        match req_ty {
            DRT::Size => 8,
            DRT::DirectionPhase => 7,
            DRT::DirectionOut => 0,
            DRT::DirectionIn => 128,
            DRT::TypePhase => 5,
            DRT::TypeClass => 32,
            DRT::TypeVendor => 64,
            DRT::RecipientInterface => 1,
            DRT::RecipientEndpoint => 2,
            DRT::RecipientOther => 3,
            DRT::Other(other) => other,
        }
    }
}

pub enum PortType {
    Standard = 0,
    Captive = 1,
    Internal = 2,
    Accessory = 3,
    ExpressCard = 4,
    Count = 5,
}
