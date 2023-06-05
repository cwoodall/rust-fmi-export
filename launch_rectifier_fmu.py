import fmpy

fmu = "data/Rectifier.fmu"
# Compile the fmu for your target
print("compiling")
fmpy.util.compile_platform_binary(filename=fmu)

print("Simulating")
start_values = {
    # variable                          start   unit      description
    'minsamplestep': (2.50000000000000000e-02, 's'),    # Minimum time step between samples in binary data file
    'binfilename':                 'DISABLED',          # Name of binary data file
}

output = [
    'outputs',  # Rectifier1.Capacitor1.v
]

result = fmpy.simulate_fmu(fmu, start_values=start_values, output=output, stop_time=0.1)


print("plotting")
fmpy.util.plot_result(result)