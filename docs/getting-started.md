Install PDM (handle pdm packages)
```
curl -sSL 
https://raw.githubusercontent.com/pdm-project/pdm/main/install-pdm.py | 
python3 -
```

Install rust: 

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Once pdm is installed, install the dependencies:

```
pdm install
```

Now you can run the project, and get some things started and underway. The first thing to do is to make sure you can run the Rectifier.fmu on your platform. I use the following [Rectifier.fmu packaged](https://github.com/modelica/fmi-cross-check/blob/master/fmus/2.0/cs/c-code/MapleSim/2018/Rectifier/Rectifier.fmu) with the source code. So you will need to run `fmpy compile`  to compile it to your platform (and have an appropriate compiler installed):

```
pdm run fmpy compile data/Rectifier.fmu
pdm run python -m fmpy.webapp data/Rectifier.fmu
```

Will launch an application a jupyter notebook has also been provided in data/Rectifier.ipynb.