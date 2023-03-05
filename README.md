# ETHDenver Hackathon Notes - Backend

# Important Packages

crypto-js is a library to encrypt the generated proof in SHA256 format to manage it in privacy mode

ethereum-cryptography to convert to bytes the IA output in an utf8ToBytes array format

ezkl is a library and command-line tool for doing inference for deep learning models and other computational graphs in a zk-snark. It enables the following workflow:
Define a computational graph, for instance a neural network (but really any arbitrary set of operations), as you would normally in pytorch or tensorflow.
https://github.com/zkonduit/ezkl

web3.js is a collection of libraries that allow you to interact with the remote zkEVM l2 in Scroll using HTTPs calls.

# Frontend Calls

## http://localhost:8080/verified-chain?helloSetter=0x18747BE67c5886881075136eb678cEADaf808028&hello=hola&signature=0x6903cb647fb3d47b91e8aecc8adc686466557d5edf96814e2b21c745f455a8502e895e696c59f8d65fd9bb57e4f202d45bb6a40c07bc8fd283d666f31264ce411b

Using to ran an publicly available neural network on some private data and produce an output that we can use to verify the proof in Scroll zkEVM


