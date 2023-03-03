import createAlchemyWeb3 from "@alch/alchemy-web3"
import dotenv from "dotenv"
import fs from "fs"
import cors from "cors"
import express from "express"
import { createRequire } from 'module';
const require = createRequire(import.meta.url);
const util = require('util');
const exec = util.promisify(require('child_process').exec);

const app = express()
dotenv.config();

const CONTRACT_ADDRESS = "0xE59da879e33b71C145b7c526a7B8C5b93195C51D"
const BACKEND_WALLET_ADDRESS = "0x18747BE67c5886881075136eb678cEADaf808028"
const JSON_CONTRACT_PATH = "./json_abi/Contract.json"
const PORT = 8080
var web3 = null
var contract = null

const loadContract = async (data) => {
    data = JSON.parse(data);

    const netId = await web3.eth.net.getId();
    contract = new web3.eth.Contract(
        data,
        CONTRACT_ADDRESS
    );
}

async function initAPI() {
    const { GOERLI_RPC_URL, PRIVATE_KEY } = process.env;
    web3 = createAlchemyWeb3.createAlchemyWeb3(GOERLI_RPC_URL);

    fs.readFile(JSON_CONTRACT_PATH, 'utf8', function (err, data) {
        if (err) {
            return console.log(err);
        }
        loadContract(data, web3)
    });

    app.listen(PORT, () => {
        console.log(`Listening to port ${PORT}`)
    })
    app.use(cors({
        origin: '*'
    }));
}

async function relaySetHello(hello, helloSetter, signature) {
    const nonce = await web3.eth.getTransactionCount(BACKEND_WALLET_ADDRESS, 'latest'); // nonce starts counting from 0
    const transaction = {
        'from': BACKEND_WALLET_ADDRESS,
        'to': CONTRACT_ADDRESS,
        'value': 0,
        'gas': 300000,
        'nonce': nonce,
        'data': contract.methods.relaySetHello(
            hello,
            helloSetter,
            signature)
            .encodeABI()
    };
    const { GOERLI_RPC_URL, PRIVATE_KEY } = process.env;
    const signedTx = await web3.eth.accounts.signTransaction(transaction, PRIVATE_KEY);

    web3.eth.sendSignedTransaction(signedTx.rawTransaction, function (error, hash) {
        if (!error) {
            console.log("🎉 The hash of your transaction is: ", hash, "\n");
        } else {
            console.log("❗Something went wrong while submitting your transaction:", error)
        }
    });
}

async function verifiedProof() {
    const { stdout, stderr } = await exec('cd ezkl && dir');
    console.log('Output was:\n', { stdout, stderr });
    return { stdout, stderr };
}

//http://localhost:8080/verified-chain
app.get('/verified-chain', (req, res) => {
    var hello = req.query["hello"]
    var helloSetter = req.query["helloSetter"]
    var message = helloSetter + " setted hello to " + " " + hello
    var list = verifiedProof()
    res.setHeader('Content-Type', 'application/json');
    res.send({
        "message": list
    })
})
initAPI()