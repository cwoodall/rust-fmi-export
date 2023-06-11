Rectifier.out.fmu: sine-fmi-sys/target/debug/libsine_fmi_sys.dylib
	cp sine-fmi-sys/target/debug/libsine_fmi_sys.dylib Rectifier.fmu/binaries/darwin64/Rectifier.dylib
	cd Rectifier.fmu && zip -r ../Rectifier.out.zip .
	mv Rectifier.out.zip Rectifier.out.fmu

sine-fmi-sys/target/debug/libsine_fmi_sys.dylib: sine-fmi-sys/src/lib.rs sine-fmi-sys/Cargo.toml
	cd sine-fmi-sys && cargo build

all: Rectifier.out.fmu

clean:
	rm -rf Rectifier.out.fmu
	rm -rf sine-fmi-sys/target

simulate: 
	fmpy simulate Rectifier.out.fmu --show-plot

.PHONY: all clean