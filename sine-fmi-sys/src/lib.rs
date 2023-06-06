use fmi2_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};

const VERSION: &str = "1.0";
const GUID: &str = "{21d9f232-b090-4c79-933f-33da939b5934}";

const FMI2TRUE: fmi2Boolean = fmi2True as fmi2Boolean;
const FMI2FALSE: fmi2Boolean = fmi2False as fmi2Boolean;

#[repr(C)]
enum ModelState {
    Instantiated,
    Initialized,
    Terminated,
    Error,
}

#[repr(C)]
struct SolverStruct {}

#[repr(C)]
struct ModelInstance {
    instanceName: fmi2String,
    GUID: fmi2String,
    fmuType: fmi2Type,
    loggingOn: fmi2Boolean,
    synced: fmi2Boolean,
    jsynced: fmi2Boolean,
    state: ModelState,
    functions: fmi2CallbackFunctions,

    tolerance: fmi2Real,
    startTime: fmi2Real,
    stopTime: fmi2Real,
    stopTimeDefined: fmi2Boolean,

    initializeMode: fmi2Boolean,
}

#[no_mangle]
pub extern "C" fn fmi2GetVersion() -> *const c_char {
    // Create a c-string from a rust string and report it
    let version = CString::new(VERSION).unwrap();
    version.into_raw()
}

#[no_mangle]
pub extern "C" fn fmi2GetTypesPlatform() -> *const c_char {
    // Create a c-string from a rust string and report it
    let version = CString::new("default").unwrap();
    version.into_raw()
}

#[no_mangle]
pub extern "C" fn fmi2SetDebugLogging(
    c: *mut c_void,
    loggingOn: fmi2Boolean,
    nCategories: usize,
    categories: *const fmi2String,
) -> fmi2Status {
    let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

    // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
    // Create a c-string from a rust string and report it

    match x.functions.logger {
        Some(f) => {
            // TODO(cw): Create a wrapper around calling this function (and other callback functions)
            let category = CString::new("log").unwrap();
            let message = CString::new("fmi2SetDebugLogging: loggingOn = %d").unwrap();
            unsafe {
                f(
                    c,
                    x.instanceName,
                    fmi2Status_fmi2OK,
                    category.into_raw(),
                    message.into_raw(),
                    loggingOn,
                );
            }
        }
        None => {
            ();
        }
    }

    match x.state {
        ModelState::Error => fmi2Status_fmi2Error,
        _ => {
            x.loggingOn = loggingOn;
            fmi2Status_fmi2OK
        }
    }
}

#[no_mangle]
pub extern "C" fn fmi2Instantiate(
    instanceName: fmi2String,
    fmuType: fmi2Type,
    fmuGUID: fmi2String,
    fmuResourceLocation: fmi2String,
    functions: fmi2CallbackFunctions,
    visible: fmi2Boolean,
    loggingOn: fmi2Boolean,
) -> *mut c_void {
    let mut x = Box::new(ModelInstance {
        instanceName: instanceName,
        GUID: fmuGUID,
        fmuType: fmuType,
        loggingOn: loggingOn,
        synced: FMI2FALSE,
        jsynced: FMI2FALSE,
        state: ModelState::Instantiated,
        functions: functions,

        tolerance: 0.0,
        startTime: 0.0,
        stopTime: 0.0,
        stopTimeDefined: FMI2FALSE,
        initializeMode: FMI2FALSE,
    });

    // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
    // Create a c-string from a rust string and report it
    let category = CString::new("log").unwrap();
    let message = CString::new("fmi2Instantiate: instanceName = %s").unwrap();
    unsafe {
        x.functions.logger.unwrap()(
            x.as_mut() as *mut ModelInstance as *mut c_void,
            instanceName,
            fmi2Status_fmi2OK,
            category.into_raw(),
            message.into_raw(),
            instanceName,
        );
    }

    Box::into_raw(x) as *mut c_void
}

// void fmi2FreeInstance(fmi2Component c) {
#[no_mangle]
pub extern "C" fn fmi2FreeInstance(c: fmi2Component) -> () {
    let mut x: Box<ModelInstance> = unsafe { Box::from_raw(c as *mut ModelInstance) };

    // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
    // Create a c-string from a rust string and report it
    let category = CString::new("log").unwrap();
    let message = CString::new("fmi2FreeInstance: instanceName = %s").unwrap();
    unsafe {
        x.functions.logger.unwrap()(
            x.as_mut() as *mut ModelInstance as *mut c_void,
            x.instanceName,
            fmi2Status_fmi2OK,
            category.into_raw(),
            message.into_raw(),
            x.instanceName,
        );
    }

    unsafe {
        x.functions.freeMemory.unwrap()(c);
    }
}

#[no_mangle]
pub extern "C" fn fmi2SetupExperiment(
    c: fmi2Component,
    toleranceDefined: fmi2Boolean,
    tolerance: fmi2Real,
    startTime: fmi2Real,
    stopTimeDefined: fmi2Boolean,
    stopTime: fmi2Real,
) -> fmi2Status {
    let mut x: Box<ModelInstance> = unsafe { Box::from_raw(c as *mut ModelInstance) };

    x.startTime = startTime;
    x.stopTime = if stopTimeDefined == FMI2TRUE {
        stopTime
    } else {
        0.0
    };

    x.stopTimeDefined = stopTimeDefined;

    fmi2Status_fmi2OK
}

#[no_mangle]
pub extern "C" fn fmi2EnterInitializationMode(c: fmi2Component) -> fmi2Status {
    // Validate that c is not a nullptr
    if c != 0 as *mut c_void {
        let mut x: Box<ModelInstance> = unsafe { Box::from_raw(c as *mut ModelInstance) };

        x.initializeMode = FMI2TRUE;
        x.jsynced = FMI2FALSE;
        x.synced = FMI2FALSE;
        x.state = ModelState::Initialized;

        fmi2Status_fmi2OK
    } else {
        fmi2Status_fmi2Error
    }

}

#[no_mangle]
pub extern "C" fn fmi2ExitInitializationMode(c: fmi2Component) -> fmi2Status {
    // Validate that c is not a nullptr
    if c != 0 as *mut c_void {
        let mut x: Box<ModelInstance> = unsafe { Box::from_raw(c as *mut ModelInstance) };

            x.initializeMode = FMI2FALSE;
            x.jsynced = FMI2FALSE;
            fmi2Status_fmi2OK
    
    } else {
        fmi2Status_fmi2Error
    }
}
#[no_mangle]
pub extern "C" fn fmi2Terminate(c: fmi2Component) -> fmi2Status {
    let mut x: Box<ModelInstance> = unsafe { Box::from_raw(c as *mut ModelInstance) };
    x.state = ModelState::Terminated;
    fmi2Status_fmi2OK
}

#[no_mangle]
pub extern "C" fn fmi2Reset(c: fmi2Component) -> fmi2Status {
    if c != 0 as *mut c_void {
        let mut x: Box<ModelInstance> = unsafe { Box::from_raw(c as *mut ModelInstance) };

        x.state = ModelState::Instantiated;
        x.jsynced = FMI2FALSE;
        x.synced = FMI2FALSE;
        fmi2Status_fmi2OK
    } else {
        fmi2Status_fmi2Error
    }
}
#[no_mangle]
pub extern "C" fn fmi2GetReal(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    value: *mut fmi2Real,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetInteger(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    value: *mut fmi2Integer,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetBoolean(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    value: *mut fmi2Boolean,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetString(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    value: *mut fmi2String,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SetReal(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    value: *const fmi2Real,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SetInteger(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    value: *const fmi2Integer,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SetBoolean(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    value: *const fmi2Boolean,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SetString(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    value: *const fmi2String,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetFMUstate(c: fmi2Component, FMUstate: *mut fmi2FMUstate) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SetFMUstate(c: fmi2Component, FMUstate: fmi2FMUstate) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2FreeFMUstate(
    c: fmi2Component,
    FMUstate: *mut fmi2FMUstate,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SerializedFMUstateSize(
    c: fmi2Component,
    FMUstate: fmi2FMUstate,
    size: *mut usize,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SerializeFMUstate(
    c: fmi2Component,
    FMUstate: fmi2FMUstate,
    serializedState: *mut fmi2Byte,
    size: usize,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2DeSerializeFMUstate(
    c: fmi2Component,
    serializedState: *const fmi2Byte,
    size: usize,
    FMUstate: *mut fmi2FMUstate,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetDirectionalDerivative(
    c: fmi2Component,
    vUnknown_ref: *const fmi2ValueReference,
    nUnknown: usize,
    vKnown_ref: *const fmi2ValueReference,
    nKnown: usize,
    dvKnown: *const fmi2Real,
    dvUnknown: *mut fmi2Real,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2EnterEventMode(c: fmi2Component) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2NewDiscreteStates(
    c: fmi2Component,
    fmi2eventInfo: *mut fmi2EventInfo,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2EnterContinuousTimeMode(c: fmi2Component) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2CompletedIntegratorStep(
    c: fmi2Component,
    noSetFMUStatePriorToCurrentPoint: fmi2Boolean,
    enterEventMode: *mut fmi2Boolean,
    terminateSimulation: *mut fmi2Boolean,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SetTime(c: fmi2Component, time: fmi2Real) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SetContinuousStates(
    c: fmi2Component,
    x: *const fmi2Real,
    nx: usize,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetDerivatives(
    c: fmi2Component,
    derivatives: *mut fmi2Real,
    nx: usize,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetEventIndicators(
    c: fmi2Component,
    eventIndicators: *mut fmi2Real,
    ni: usize,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetContinuousStates(
    c: fmi2Component,
    x: *mut fmi2Real,
    nx: usize,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetNominalsOfContinuousStates(
    c: fmi2Component,
    x_nominal: *mut fmi2Real,
    nx: usize,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SetRealInputDerivatives(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    order: *const fmi2Integer,
    value: *const fmi2Real,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetRealOutputDerivatives(
    c: fmi2Component,
    vr: *const fmi2ValueReference,
    nvr: usize,
    order: *const fmi2Integer,
    value: *mut fmi2Real,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2DoStep(
    c: fmi2Component,
    currentCommunicationPoint: fmi2Real,
    communicationStepSize: fmi2Real,
    noSetFMUStatePriorToCurrentPoint: fmi2Boolean,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2CancelStep(c: fmi2Component) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetStatus(
    c: fmi2Component,
    s: fmi2StatusKind,
    value: *mut fmi2Status,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetRealStatus(
    c: fmi2Component,
    s: fmi2StatusKind,
    value: *mut fmi2Real,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetIntegerStatus(
    c: fmi2Component,
    s: fmi2StatusKind,
    value: *mut fmi2Integer,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetBooleanStatus(
    c: fmi2Component,
    s: fmi2StatusKind,
    value: *mut fmi2Boolean,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetStringStatus(
    c: fmi2Component,
    s: fmi2StatusKind,
    value: *mut fmi2String,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
