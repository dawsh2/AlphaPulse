const AMOY_CONFIG = {
    rpcUrl: 'https://rpc-amoy.polygon.technology',
    chainId: 80002,
    gasPrice: ethers.utils.parseUnits('30', 'gwei'), // Higher for Amoy
    
    // Amoy token addresses (different from Mumbai)
    tokens: {
        USDC: '0x41E94Eb019C0762f9Bfcf9Fb1E58725BfB0e7582', // Amoy USDC
        WMATIC: '0x360ad4f9a9A8EFe9A8DCB5f461c4Cc1047E1Dcf9', // Amoy WMATIC
        WETH: '0x7ceB23fD6eC88b87c7e50c3D0d0c18d8b4e7d0f32', // Amoy WETH
    },
    
    // Amoy DEX addresses (need to be updated)
    aavePool: '0x1C4a4e31231F71Fc34867D034a9E68f6fC798249', // Amoy Aave
};

module.exports = { AMOY_CONFIG };
