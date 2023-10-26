all: SineModel.out.fmu

Sine.out.fmu: sine-fmi-sys/target/debug/libsine_fmi_sys.dylib sine-fmi-sys/Sine.fmu/modelDescription.xml
	cp sine-fmi-sys/target/debug/libsine_fmi_sys.dylib sine-fmi-sys/Sine.fmu/binaries/darwin64/Sine.dylib
	cd sine-fmi-sys/Sine.fmu && zip -r ../../Sine.out.zip .
	mv Sine.out.zip Sine.out.fmu

sine-fmi-sys/target/debug/libsine_fmi_sys.dylib: sine-fmi-sys/src/lib.rs sine-fmi-sys/Cargo.toml
	cd sine-fmi-sys && cargo build

sine-fmi-sys/Sine.fmu/modelDescription.xml:

SineModel.out.fmu: sine-derive-fmi/SineModel.zip
	mv sine-derive-fmi/SineModel.zip ./SineModel.out.fmu

sine-derive-fmi/SineModel.zip: sine-derive-fmi/target/debug/libsine_derive_fmi.dylib

sine-derive-fmi/target/debug/libsine_derive_fmi.dylib: sine-derive-fmi/src/lib.rs sine-derive-fmi/Cargo.toml
	cd sine-derive-fmi && cargo build && cargo create-fmu

clean:
	rm -rf Sine.out.fmu
	rm -rf SineModel.out.fmu
	rm -rf sine-fmi-sys/target
	rm -rf sine-derive-fmi/target

simulate: 
	fmpy simulate Sine.out.fmu --show-plot

.PHONY: all clean