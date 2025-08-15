require("@nomiclabs/hardhat-ethers");
require("@nomiclabs/hardhat-etherscan");
require("dotenv").config();

module.exports = {
    solidity: {
        version: "0.8.19",
        settings: {
            optimizer: {
                enabled: true,
                runs: 200
            }
        }
    },
    networks: {
        polygon: {
            url: process.env.POLYGON_RPC_URL || "https://polygon-mainnet.public.blastapi.io",
            accounts: process.env.PRIVATE_KEY ? [process.env.PRIVATE_KEY] : [],
            gasPrice: 50000000000 // 50 gwei
        },
        polygonMumbai: {
            url: process.env.MUMBAI_RPC_URL || "https://rpc-mumbai.maticvigil.com",
            accounts: process.env.PRIVATE_KEY ? [process.env.PRIVATE_KEY] : [],
            gasPrice: 30000000000 // 30 gwei
        }
    },
    etherscan: {
        apiKey: {
            polygon: process.env.POLYGONSCAN_API_KEY || "",
            polygonMumbai: process.env.POLYGONSCAN_API_KEY || ""
        }
    }
};