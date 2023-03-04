import dotenv from "dotenv"
import fs from "fs"
import web3 from "web3"
import cors from "cors"
import express from "express"
import { createRequire } from 'module';
import { getWeb3, getContract } from './utils/web3.js';

const require = createRequire(import.meta.url);
const util = require('util');
const exec = util.promisify(require('child_process').exec);

const app = express()
dotenv.config();
const { GOERLI_RPC_URL, PRIVATE_KEY } = process.env;
const CONTRACT_ADDRESS = "0xD0ce6D448227F2C5239116Be26a1bB91BfB1c326"
const JSON_CONTRACT_PATH = "./json_abi/Contract.json"
const PORT = 8080
let provider = "null";
let contract = "null";

const loadContract = async (data) => {
    // Load contract
    provider = getWeb3(GOERLI_RPC_URL);
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
        var hello = "b221d9dbb083a7f33428d7c2a3c3198ae925614d70210e28716ccaa7cd4ddb79"
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
        const txCreateProof = contract.methods.create_proof(`0x${hello}`);

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

async function verifiedProof() {
    // Execute command
    const { stdout, stderr } = await exec('cd ezkl && ezkl -K=17 gen-srs --params-path=kzg.params && ezkl --bits=16 -K=17 prove -D ./examples/onnx/1l_relu/input.json -M ./examples/onnx/1l_relu/network.onnx --proof-path 1l_relu.pf --vk-path 1l_relu.vk --params-path=kzg.params');
    console.log('Output was:\n', { stdout, stderr });

    // Verify proof
    const verifyContent = fs.readFileSync('./ezkl/1l_relu.pf').toString()

    // Send proof to relayer to verify and store on chain 
    await relaySetProof(verifyContent);

    return { stdout, stderr };
}

//http://localhost:8080/verified-chain
app.get('/verified-chain', async (req, res) => {
    var list = await verifiedProof();
    res.setHeader('Content-Type', 'application/json');
    res.send({
        "message": list
    })
})
initAPI()