// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

/// @title FlashArbitrageOptimized - Gas-optimized baseline for Huff comparison
/// @notice This contract serves as the optimized Solidity baseline before Huff migration
/// @dev Uses assembly optimizations in critical paths for maximum gas efficiency
contract FlashArbitrageOptimized {
    address private immutable owner;
    
    // Packed struct for gas efficiency (fits in 2 storage slots)
    struct SwapParams {
        address tokenIn;        // 20 bytes
        address tokenOut;       // 20 bytes  
        address router;         // 20 bytes
        uint128 amountIn;       // 16 bytes (packed to 256 bits)
        uint24 fee;             // 3 bytes (V3 fee tier)
        bool isV3;              // 1 byte (router type flag)
        // Total: 80 bytes = 3 storage slots (optimized)
    }
    
    // Aave V3 on Polygon (immutable for gas savings)
    address private constant ADDRESSES_PROVIDER = 0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb;
    address private constant POOL_ADDRESS = 0x794a61358D6845594F94dc1DB02A252b5b4814aD; // Pre-computed
    
    // Token addresses (immutable constants)
    address private constant WPOL = 0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270;
    address private constant USDC_OLD = 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174;
    address private constant USDC_NEW = 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359;
    
    // Router addresses (immutable constants)
    address private constant SUSHISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    address private constant BALANCER_VAULT = 0xBA12222222228d8Ba445958a75a0704d566BF2C8;
    
    // Function selectors for gas optimization
    bytes4 private constant APPROVE_SELECTOR = 0x095ea7b3;
    bytes4 private constant SWAP_EXACT_TOKENS_SELECTOR = 0x38ed1739;
    bytes4 private constant FLASHLOAN_SELECTOR = 0xab9c4b5d;
    
    // Gas tracking for baseline measurement
    uint256 public lastGasUsed;
    uint256 public totalGasUsed;
    uint256 public executionCount;
    
    error NotOwner();
    error InsufficientProfit();
    error SwapFailed();
    error FlashLoanFailed();
    
    modifier onlyOwner() {
        assembly {
            if iszero(eq(caller(), sload(0))) {
                mstore(0, 0x30cd747100000000000000000000000000000000000000000000000000000000) // NotOwner()
                revert(0, 4)
            }
        }
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    /// @notice Execute optimized flash loan arbitrage
    /// @param flashAmount Amount to borrow via flash loan
    /// @param minProfit Minimum profit required (wei)
    function executeArbitrage(uint256 flashAmount, uint256 minProfit) external onlyOwner {
        uint256 gasStart = gasleft();
        
        // Setup flash loan with optimized parameters
        _executeFlashLoan(flashAmount, minProfit);
        
        // Track gas usage for baseline comparison
        uint256 gasUsed = gasStart - gasleft();
        lastGasUsed = gasUsed;
        totalGasUsed += gasUsed;
        executionCount++;
    }
    
    /// @notice Execute flash loan with assembly optimizations
    function _executeFlashLoan(uint256 amount, uint256 minProfit) internal {
        assembly {
            // Prepare flash loan calldata in memory
            let ptr := mload(0x40) // Free memory pointer
            
            // Function selector: flashLoan(address,address[],uint256[],uint256[],address,bytes,uint16)
            mstore(ptr, FLASHLOAN_SELECTOR)
            
            // receiverAddress = this
            mstore(add(ptr, 0x04), address())
            
            // assets array offset
            mstore(add(ptr, 0x24), 0xe0)
            
            // amounts array offset  
            mstore(add(ptr, 0x44), 0x120)
            
            // modes array offset
            mstore(add(ptr, 0x64), 0x160)
            
            // onBehalfOf = this
            mstore(add(ptr, 0x84), address())
            
            // params offset
            mstore(add(ptr, 0xa4), 0x1a0)
            
            // referralCode = 0
            mstore(add(ptr, 0xc4), 0)
            
            // Assets array: [USDC_OLD]
            mstore(add(ptr, 0xe0), 1) // length
            mstore(add(ptr, 0x100), USDC_OLD)
            
            // Amounts array: [amount]
            mstore(add(ptr, 0x120), 1) // length
            mstore(add(ptr, 0x140), amount)
            
            // Modes array: [0]
            mstore(add(ptr, 0x160), 1) // length
            mstore(add(ptr, 0x180), 0)
            
            // Params (encode minProfit)
            mstore(add(ptr, 0x1a0), 0x20) // length of bytes
            mstore(add(ptr, 0x1c0), minProfit)
            
            // Update free memory pointer
            mstore(0x40, add(ptr, 0x1e0))
            
            // Execute flash loan call
            let success := call(gas(), POOL_ADDRESS, 0, ptr, 0x1e0, 0, 0)
            if iszero(success) {
                mstore(0, 0x7939f42400000000000000000000000000000000000000000000000000000000) // FlashLoanFailed()
                revert(0, 4)
            }
        }
    }
    
    /// @notice Optimized flash loan callback with assembly
    /// @dev Called by Aave pool during flash loan execution
    function executeOperation(
        address[] calldata assets,
        uint256[] calldata amounts,
        uint256[] calldata premiums,
        address initiator,
        bytes calldata params
    ) external returns (bool) {
        // Verify caller and initiator
        require(msg.sender == POOL_ADDRESS, "Invalid caller");
        require(initiator == address(this), "Invalid initiator");
        
        uint256 amountBorrowed = amounts[0];
        uint256 premium = premiums[0];
        uint256 minProfit = abi.decode(params, (uint256));
        
        // Execute arbitrage with assembly optimizations
        uint256 finalAmount = _executeArbitrageLogic(amountBorrowed);
        
        // Check profitability
        uint256 totalDebt = amountBorrowed + premium;
        if (finalAmount < totalDebt + minProfit) {
            revert InsufficientProfit();
        }
        
        // Approve repayment with assembly
        _approveToken(USDC_OLD, POOL_ADDRESS, totalDebt);
        
        // Transfer profit to owner
        if (finalAmount > totalDebt) {
            uint256 profit = finalAmount - totalDebt;
            _transferToken(USDC_OLD, owner, profit);
        }
        
        return true;
    }
    
    /// @notice Execute arbitrage logic with assembly optimizations
    function _executeArbitrageLogic(uint256 amountIn) internal returns (uint256 finalAmount) {
        // Step 1: Approve and swap USDC_OLD -> WPOL
        _approveToken(USDC_OLD, SUSHISWAP_ROUTER, amountIn);
        uint256 wpolReceived = _executeSwapOptimized(USDC_OLD, WPOL, amountIn, SUSHISWAP_ROUTER);
        
        // Step 2: Approve and swap WPOL -> USDC_NEW  
        _approveToken(WPOL, SUSHISWAP_ROUTER, wpolReceived);
        finalAmount = _executeSwapOptimized(WPOL, USDC_NEW, wpolReceived, SUSHISWAP_ROUTER);
    }
    
    /// @notice Assembly-optimized token approval
    function _approveToken(address token, address spender, uint256 amount) internal {
        assembly {
            let ptr := mload(0x40)
            mstore(ptr, APPROVE_SELECTOR)
            mstore(add(ptr, 0x04), spender)
            mstore(add(ptr, 0x24), amount)
            
            let success := call(gas(), token, 0, ptr, 0x44, 0, 0)
            if iszero(success) { revert(0, 0) }
        }
    }
    
    /// @notice Assembly-optimized token transfer
    function _transferToken(address token, address to, uint256 amount) internal {
        assembly {
            let ptr := mload(0x40)
            mstore(ptr, 0xa9059cbb00000000000000000000000000000000000000000000000000000000) // transfer selector
            mstore(add(ptr, 0x04), to)
            mstore(add(ptr, 0x24), amount)
            
            let success := call(gas(), token, 0, ptr, 0x44, 0, 0)
            if iszero(success) { revert(0, 0) }
        }
    }
    
    /// @notice Assembly-optimized swap execution
    function _executeSwapOptimized(
        address tokenIn,
        address tokenOut, 
        uint256 amountIn,
        address router
    ) internal returns (uint256 amountOut) {
        assembly {
            let ptr := mload(0x40)
            
            // Function selector: swapExactTokensForTokens
            mstore(ptr, SWAP_EXACT_TOKENS_SELECTOR)
            
            // amountIn
            mstore(add(ptr, 0x04), amountIn)
            
            // amountOutMin = 0
            mstore(add(ptr, 0x24), 0)
            
            // path offset
            mstore(add(ptr, 0x44), 0xa0)
            
            // to = this
            mstore(add(ptr, 0x64), address())
            
            // deadline = block.timestamp + 300
            mstore(add(ptr, 0x84), add(timestamp(), 300))
            
            // Path array
            mstore(add(ptr, 0xa0), 2) // length = 2
            mstore(add(ptr, 0xc0), tokenIn)
            mstore(add(ptr, 0xe0), tokenOut)
            
            // Execute call and capture return data
            let success := call(gas(), router, 0, ptr, 0x100, ptr, 0x40)
            if iszero(success) {
                mstore(0, 0x7c2f2d0100000000000000000000000000000000000000000000000000000000) // SwapFailed()
                revert(0, 4)
            }
            
            // Extract amountOut from return data (second element of amounts array)
            amountOut := mload(add(ptr, 0x20))
        }
    }
    
    /// @notice Get execution statistics for baseline comparison
    function getExecutionStats() external view returns (
        uint256 avgGasUsed,
        uint256 lastGas,
        uint256 totalGas,
        uint256 execCount
    ) {
        avgGasUsed = executionCount > 0 ? totalGasUsed / executionCount : 0;
        lastGas = lastGasUsed;
        totalGas = totalGasUsed;
        execCount = executionCount;
    }
    
    /// @notice Check profitability before execution
    function checkProfitability(uint256 flashAmount) external view returns (
        uint256 expectedWpol,
        uint256 expectedUsdcNew,
        uint256 flashFee,
        uint256 netProfit,
        bool isProfitable
    ) {
        // Calculate flash loan fee (0.05% on Aave V3)
        flashFee = (flashAmount * 5) / 10000;
        
        // Simulate swaps (view-only calls)
        expectedWpol = _simulateSwap(USDC_OLD, WPOL, flashAmount, SUSHISWAP_ROUTER);
        expectedUsdcNew = _simulateSwap(WPOL, USDC_NEW, expectedWpol, SUSHISWAP_ROUTER);
        
        // Calculate net profit
        uint256 totalDebt = flashAmount + flashFee;
        isProfitable = expectedUsdcNew > totalDebt;
        netProfit = isProfitable ? expectedUsdcNew - totalDebt : 0;
    }
    
    /// @notice Simulate swap for profitability calculation
    function _simulateSwap(
        address tokenIn,
        address tokenOut,
        uint256 amountIn,
        address router
    ) internal view returns (uint256 amountOut) {
        // This would call getAmountsOut on the router
        // Simplified for baseline - actual implementation would use router.getAmountsOut()
        // For now, assume 1:1 ratio for compilation
        return amountIn;
    }
    
    /// @notice Emergency withdrawal function
    function withdraw(address token) external onlyOwner {
        assembly {
            // Get token balance
            let ptr := mload(0x40)
            mstore(ptr, 0x70a0823100000000000000000000000000000000000000000000000000000000) // balanceOf selector
            mstore(add(ptr, 0x04), address())
            
            let success := staticcall(gas(), token, ptr, 0x24, ptr, 0x20)
            if iszero(success) { revert(0, 0) }
            
            let balance := mload(ptr)
            if gt(balance, 0) {
                // Transfer balance to owner
                mstore(ptr, 0xa9059cbb00000000000000000000000000000000000000000000000000000000) // transfer selector
                mstore(add(ptr, 0x04), sload(0)) // owner from storage slot 0
                mstore(add(ptr, 0x24), balance)
                
                success := call(gas(), token, 0, ptr, 0x44, 0, 0)
                if iszero(success) { revert(0, 0) }
            }
        }
    }
    
    receive() external payable {}
}