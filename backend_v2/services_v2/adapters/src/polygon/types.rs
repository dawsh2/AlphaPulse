//! Polygon-specific types and constants

use web3::types::H256;

/// Known Polygon DEX factory addresses
pub const UNISWAP_V2_FACTORY: &str = "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32";
pub const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
pub const QUICKSWAP_FACTORY: &str = "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32";

/// Event topic signatures
pub mod topics {
    use web3::types::H256;

    // Uniswap V2 Events
    pub const V2_SWAP_TOPIC: &str =
        "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
    pub const V2_SYNC_TOPIC: &str =
        "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";
    pub const V2_MINT_TOPIC: &str =
        "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26394f4c03821c4f";
    pub const V2_BURN_TOPIC: &str =
        "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496";

    // Uniswap V3 Events
    pub const V3_SWAP_TOPIC: &str =
        "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
    pub const V3_MINT_TOPIC: &str =
        "0x7a53080ba414158be7ec69b987b5fb7d07dee101bfd5d6f8d951e2e0e5b43b25";
    pub const V3_BURN_TOPIC: &str =
        "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c";
    pub const V3_TICK_TOPIC: &str =
        "0xb0c3ac81a86404a07941a9e2e6b6fe5eb8902be394e606de7efcb7e0dd10fd1b";
}

/// Polygon-specific configuration
#[derive(Debug, Clone)]
pub struct PolygonNetworkConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub ws_url: String,
    pub block_confirmations: u64,
    pub batch_size: usize,
}
