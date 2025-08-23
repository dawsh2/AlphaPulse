const MAINNET_CONFIG = {
    rpcUrl: 'https://polygon-rpc.com',
    chainId: 137,
    gasPrice: ethers.utils.parseUnits('30', 'gwei'), // 30 gwei
    
    // Mainnet addresses
    tokens: {
        USDC: '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174', // USDC.e
        WMATIC: '0x0d500B1d8E8eF31E21C99d1db9A6444d3ADf1270', // WMATIC
        WETH: '0x7ceB23fD6eC88b87c7e50c3D0d0c18d8b4e7d0f32', // WETH
    },
    
    // Mainnet DEX addresses
    quickswap: '0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff',
    sushiswap: '0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506',
    aavePool: '0x794a61358D6845594F94dc1DB02A252b5b4814aD', // Aave V3
};

module.exports = { MAINNET_CONFIG };
