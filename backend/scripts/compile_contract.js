const fs = require('fs');
const solc = require('solc');

const source = fs.readFileSync('FastFlashArb.sol', 'utf8');

const input = {
    language: 'Solidity',
    sources: {
        'FastFlashArb.sol': {
            content: source
        }
    },
    settings: {
        outputSelection: {
            '*': {
                '*': ['*']
            }
        }
    }
};

const output = JSON.parse(solc.compile(JSON.stringify(input)));

if (output.errors) {
    console.error('Compilation errors:', output.errors);
}

const contract = output.contracts['FastFlashArb.sol']['FastFlashArb'];
console.log('BYTECODE =', '"0x' + contract.evm.bytecode.object + '"');
console.log('ABI =', JSON.stringify(contract.abi));
