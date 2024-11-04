var path = require('path');
var fs = require('fs');
var bcoin = require('bcoin');
var BlockStore = require('./node_modules/bcoin/lib/blockstore/level');

class Queue {
    constructor(limit, size = 0, queue = []) {
        this.limit = limit;
        this.size = size;
        this.queue = queue
    }

    push(tx_hash, block_hash) {
        this.queue.unshift([tx_hash, block_hash]);
        if (this.size + 1 <= this.limit) this.size++;
        else this.queue.pop();
    }

    get() {
        return this.queue;
    }
}

// Magic Number
const separator = Buffer.from([0xFA, 0xBF, 0xB5, 0xDA]);

// Network set to regtest
const network = bcoin.Network.get('regtest');

const mocktime = 1637053432;

network.time.offset = 0;

const delta = network.now() - mocktime;

network.time.offset = -delta;

// Initialize blockchain
const blocks = new BlockStore({
  memory: true,
  network
});

const workers = new bcoin.WorkerPool({
    enabled: true,
    size: 2
});

const chain = new bcoin.Chain({
  memory: true,
  blocks,
  network,
  workers
});

// Start fuzzing
startFuzzing('./import/import.dat', './test_cases');

async function startFuzzing(dat_path, dir_path) {

    // Import .dat
    const imported_blocks = splitBuffer(fs.readFileSync(dat_path), separator);
    const imported_txos = [];

    // Open blockchain
    await blocks.open();
    await workers.open();
    await chain.open();

    // Add imported blocks to blockchain and save TXO
    for (block_raw of imported_blocks.slice(1)) {
        const block = bcoin.Block.fromRaw(block_raw.slice(4)); // Assuming that each block starts with 4 bytes size
        await chain.add(block);
        block.txs.forEach((tx) => imported_txos.push([tx.hash(), tx.outputs.length]));
    }
    
    // Import test cases
    const test_cases = fs.readdirSync(dir_path);

    // Fuzz and write down the result
    for (const test_case of test_cases) {
        console.log(test_case);
        await chain.reset(imported_blocks.length - 1);
        fs.writeFileSync(path.join("./output", test_case), JSON.stringify(await _fuzz(path.join(dir_path, test_case), imported_txos), null, 4));
    }

    // Debug
    console.log("finished");

    // Close blockchain
    await blocks.close();
    await workers.close();
    await chain.close();
}

 async function _fuzz(file_path, imported_txos) {

    // Import test cases
    const test_blocks = splitBuffer(fs.readFileSync(file_path), separator).map(block => block.slice(9)); // Assuming that each test case starts with 9 bytes info

    let result = {};

    const txos = [];

    const coinbase = new Queue(100);

    const utxos = new Set();

    // Try to add blocks to the chain
    for (const [index, block_raw] of test_blocks.entries()) {
        let hash = null;
        let reason = 'Valid';

        try {
            let block = bcoin.Block.fromRaw(block_raw);
            hash = block.hash();
            await chain.add(block);
            block.txs.forEach((tx) => txos.push([tx.hash(), tx.outputs.length]));
            coinbase.push(bcoin.util.revHex(block.txs[0].hash()), block.hash());
        } catch (e) {
            reason = e.message;
        }

        result['Block ' + padStringWithZeros(index.toString(), 2)] = { 
            'accept': !!((hash) && (await chain.getEntry(hash) === chain.tip)),
            'reason': reason
        };

        for (const txo of coinbase.get()) {
            if (await chain.isMainHash(txo[1])) utxos.add(txo[0]);
        }
    }

    result['HashTip'] = chain.tip.rhash();

    // Save UTXO
    for (const txo of imported_txos.concat(txos)) {
        for (let i = 0; i < txo[1]; i++) {
            const utxo = await chain.getCoin(txo[0], i);
            if (utxo) {
                utxos.add(utxo.rhash());
                break;
            }
        }
    }

    result["UTXO"] = Array.from(utxos).sort();
    
    return result;
}

function splitBuffer(data, separator) {
    let splitted_array = new Array();
    for (let i = data.readInt32BE() === separator.readInt32BE() ? 4 : 0, chunk_start = i; i <= data.length - 4; i++) {
        if (data.readInt32BE(i) === separator.readInt32BE()) { 
            splitted_array.push(Buffer.from(data.slice(chunk_start, i)))
            chunk_start = i + 4;
        } else if (i === data.length - 4) {
            splitted_array.push(Buffer.from(data.slice(chunk_start, data.length)));
        }
    }
    return splitted_array;
}

function padStringWithZeros(inputString, desiredLength) {
    if (inputString.length >= desiredLength) {
      return inputString; // No need to pad if the string is already as long as desired.
    }
  
    const zerosToPad = desiredLength - inputString.length;
    const zeros = '0'.repeat(zerosToPad);
    
    return zeros + inputString;
  }