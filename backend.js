import dotenv from "dotenv"
import fs from "fs"
import web3 from "web3"
import cors from "cors"
import express from "express"
import { createRequire } from 'module';
import { getWeb3, getContract } from './utils/web3.js';
import { hashMessage } from "./utils/hashMessaje.js"

const require = createRequire(import.meta.url);
const util = require('util');
const exec = util.promisify(require('child_process').exec);

const app = express()
dotenv.config();
const { SCROLL_ALPHA_RPC_URL, PRIVATE_KEY } = process.env;
const CONTRACT_ADDRESS = "0x7897e2050b129DC4934fe815807FC0Fe1C291194"
const JSON_CONTRACT_PATH = "./json_abi/Contract.json"
const PORT = 8080
let provider = "";
let contract = "";

const loadContract = async (data) => {
    // Load contract
    provider = getWeb3(SCROLL_ALPHA_RPC_URL);
    const abi = JSON.parse(data);
    contract = getContract(provider, abi, CONTRACT_ADDRESS);
}

async function initAPI() {
    // Read contract
    fs.readFile(JSON_CONTRACT_PATH, 'utf8', function (err, data) {
        if (err) {
            return console.log(err);
        }
        loadContract(data)
    });

    // Start server
    app.listen(PORT, () => {
        console.log(`Listening to port ${PORT}`)
    })

    // Enable CORS
    app.use(cors({
        origin: '*'
    }));
}

async function relaySetProof(hello) {
    try {
        // Create account from key
        const account = provider.eth.accounts.privateKeyToAccount(PRIVATE_KEY);

        console.log(account)
        web3.utils.toChecksumAddress(account.address)
        web3.utils.isAddress(account.address)
        // Add account to wallet
        provider.eth.accounts.wallet.add(account);
        provider.eth.defaultAccount = account.address;
        console.log(provider.eth.defaultAccount)
        // Create transaction
        const txCreateProof = contract.methods.create_proof(hello);

        // Estimate gas
        const [gasPrice, gasCost1] = await Promise.all([
            provider.eth.getGasPrice(),
            txCreateProof.estimateGas({ from: provider.eth.defaultAccount }),
        ]);

        // Send transaction
        const dataTx = txCreateProof.encodeABI();
        const txData = {
            to: contract.options.address,
            data: dataTx,
            gas: gasCost1,
            gasPrice,
        };

        // Send transaction
        const receipt = await provider.eth.sendTransaction(txData);
        console.log(receipt);
    } catch (error) {
        console.log(error);
    }
}

async function proof(hello, helloSetter, signature) {
    // Execute command
    const { stdout, stderr } = await exec('cd ezkl && ezkl -K=17 gen-srs --params-path=kzg.params && ezkl --bits=16 -K=17 prove -D ./examples/onnx/1l_relu/input.json -M ./examples/onnx/1l_relu/network.onnx --proof-path 1l_relu.pf --vk-path 1l_relu.vk --params-path=kzg.params');
    console.log('Output was:\n', { stdout, stderr, hello, helloSetter, signature });
    // Verify proof
    const verifyContent = fs.readFileSync('./ezkl/1l_relu.pf').toString()

    // Write proof to file
    fs.writeFileSync('test.txt', verifyContent);
    const message = hashMessage(verifyContent)

    console.log(message)
    // Send proof to relayer to verify and store on chain 
    await relaySetProof(message);

    return { stdout, stderr, message };
}

async function verifyProof() {
    try {
        // Create account from key
        const account = provider.eth.accounts.privateKeyToAccount(PRIVATE_KEY);

        web3.utils.toChecksumAddress(account.address)
        web3.utils.isAddress(account.address)
        // Add account to wallet
        provider.eth.accounts.wallet.add(account);
        provider.eth.defaultAccount = account.address;

        // Get counter
        const weiCounter = await contract.methods.counter.call().call();

        // Create transaction
        const verifyContent = fs.readFileSync('./ezkl/1l_relu.pf').toString()
        const message = hashMessage(verifyContent)
        const txCreateProof = await contract.methods.verify_proof(weiCounter - 1, message).call();
        console.log(txCreateProof);
    } catch (error) {
        console.log("My error", error);
    }
}


//http://localhost:8080/verified-chain?helloSetter=0x18747BE67c5886881075136eb678cEADaf808028&hello=hola&signature=0x6903cb647fb3d47b91e8aecc8adc686466557d5edf96814e2b21c745f455a8502e895e696c59f8d65fd9bb57e4f202d45bb6a40c07bc8fd283d666f31264ce411b
app.get('/verified-chain', async (req, res) => {
    var hello = req.query["hello"]
    var helloSetter = req.query["helloSetter"]
    var signature = req.query["signature"]
    var message = helloSetter + " setted hello to " + " " + hello
    await proof(hello, helloSetter, signature);
    await verifyProof();
    res.setHeader('Content-Type', 'application/json');
    res.send({
        "message": message
    })
})
initAPI()