Sine.out.fmu: sine-fmi-sys/target/debug/libsine_fmi_sys.dylib sine-fmi-sys/Sine.fmu/modelDescription.xml
	cp sine-fmi-sys/target/debug/libsine_fmi_sys.dylib sine-fmi-sys/Sine.fmu/binaries/darwin64/Sine.dylib
	cd sine-fmi-sys/Sine.fmu && zip -r ../../Sine.out.zip .
	mv Sine.out.zip Sine.out.fmu

sine-fmi-sys/target/debug/libsine_fmi_sys.dylib: sine-fmi-sys/src/lib.rs sine-fmi-sys/Cargo.toml
	cd sine-fmi-sys && cargo build

all: Sine.out.fmu

sine-fmi-sys/Sine.fmu/modelDescription.xml:

clean:
	rm -rf Sine.out.fmu
	rm -rf sine-fmi-sys/target

simulate: 
	fmpy simulate Sine.out.fmu --show-plot

.PHONY: all clean