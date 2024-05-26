let conf_file = process.env.CONFIG;
let config = require(conf_file);


let zmq = require("zeromq");
let sock = zmq.socket("pull");
let { FlashbotsBundleProvider } = require("@flashbots/ethers-provider-bundle");
let { JsonRpcProvider, Wallet } = require("ethers");
sock.connect(`tcp://127.0.0.1:${config.port}`);

const provider = new JsonRpcProvider(config.ethereum_rpc, 1);
const wallet = Wallet.createRandom(); // TODO: import key
const flashbotsProvider = (async () => await FlashbotsBundleProvider.create(provider, wallet))();

console.log("Worker connected to port 3000");

sock.on("message", async function (msg) {
    let data = JSON.parse(msg.toString());

    let hash = data.hash.bundle_hash;
    let block = data.block;

    let result = await flashbotsProvider.getConflictingBundle([hash], block);

    console.log(result);
    // TODO: extract data to TG
})