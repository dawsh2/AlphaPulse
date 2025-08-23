"""
DEX Quoter Service - Get real quotes from DEX routers
"""
from typing import Dict, List, Tuple, Optional
from web3 import Web3
import json
import logging

logger = logging.getLogger(__name__)

# Polygon mainnet addresses
UNISWAP_V3_QUOTER = "0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6"
UNISWAP_V2_ROUTER = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff"  # QuickSwap router

class DexQuoter:
    """Get real quotes from DEX quoter contracts"""
    
    def __init__(self, w3: Web3):
        self.w3 = w3
        self.quoter_v3 = self._init_v3_quoter()
        self.router_v2 = self._init_v2_router()
        
    def _init_v3_quoter(self):
        """Initialize Uniswap V3 Quoter contract"""
        quoter_abi = [
            {
                "inputs": [
                    {"name": "tokenIn", "type": "address"},
                    {"name": "tokenOut", "type": "address"},
                    {"name": "fee", "type": "uint24"},
                    {"name": "amountIn", "type": "uint256"},
                    {"name": "sqrtPriceLimitX96", "type": "uint160"}
                ],
                "name": "quoteExactInputSingle",
                "outputs": [{"name": "amountOut", "type": "uint256"}],
                "stateMutability": "nonpayable",
                "type": "function"
            }
        ]
        return self.w3.eth.contract(
            address=self.w3.to_checksum_address(UNISWAP_V3_QUOTER),
            abi=quoter_abi
        )
    
    def _init_v2_router(self):
        """Initialize Uniswap V2 Router contract"""
        router_abi = [
            {
                "inputs": [
                    {"name": "amountIn", "type": "uint256"},
                    {"name": "path", "type": "address[]"}
                ],
                "name": "getAmountsOut",
                "outputs": [{"name": "amounts", "type": "uint256[]"}],
                "stateMutability": "view",
                "type": "function"
            }
        ]
        return self.w3.eth.contract(
            address=self.w3.to_checksum_address(UNISWAP_V2_ROUTER),
            abi=router_abi
        )
    
    async def get_v3_quote(
        self,
        token_in: str,
        token_out: str,
        amount_in: int,
        fee: int = 3000  # 0.3% fee tier
    ) -> Dict:
        """
        Get quote from Uniswap V3
        
        Args:
            token_in: Input token address
            token_out: Output token address  
            amount_in: Amount in smallest unit
            fee: Fee tier (500, 3000, 10000)
            
        Returns:
            Quote result with amount out and price impact
        """
        try:
            # Call quoter contract
            amount_out = self.quoter_v3.functions.quoteExactInputSingle(
                self.w3.to_checksum_address(token_in),
                self.w3.to_checksum_address(token_out),
                fee,
                amount_in,
                0  # No price limit
            ).call()
            
            # Calculate effective price and slippage
            effective_price = amount_out / amount_in if amount_in > 0 else 0
            
            return {
                "protocol": "UNISWAP_V3",
                "amountIn": amount_in,
                "amountOut": amount_out,
                "price": effective_price,
                "fee": fee / 10000,  # Convert to percentage
                "success": True
            }
            
        except Exception as e:
            logger.error(f"V3 quote failed: {e}")
            return {
                "protocol": "UNISWAP_V3",
                "amountIn": amount_in,
                "amountOut": 0,
                "price": 0,
                "fee": fee / 10000,
                "success": False,
                "error": str(e)
            }
    
    async def get_v2_quote(
        self,
        token_in: str,
        token_out: str,
        amount_in: int
    ) -> Dict:
        """
        Get quote from Uniswap V2 style DEX
        
        Args:
            token_in: Input token address
            token_out: Output token address
            amount_in: Amount in smallest unit
            
        Returns:
            Quote result with amount out and price impact
        """
        try:
            # Build path for swap
            path = [
                self.w3.to_checksum_address(token_in),
                self.w3.to_checksum_address(token_out)
            ]
            
            # Get amounts out from router
            amounts = self.router_v2.functions.getAmountsOut(
                amount_in,
                path
            ).call()
            
            amount_out = amounts[1]  # Second element is output amount
            effective_price = amount_out / amount_in if amount_in > 0 else 0
            
            return {
                "protocol": "UNISWAP_V2",
                "amountIn": amount_in,
                "amountOut": amount_out,
                "price": effective_price,
                "fee": 0.003,  # 0.3% fixed fee
                "success": True
            }
            
        except Exception as e:
            logger.error(f"V2 quote failed: {e}")
            return {
                "protocol": "UNISWAP_V2",
                "amountIn": amount_in,
                "amountOut": 0,
                "price": 0,
                "fee": 0.003,
                "success": False,
                "error": str(e)
            }
    
    async def get_best_quote(
        self,
        token_in: str,
        token_out: str,
        amount_in: int
    ) -> Dict:
        """
        Get best quote across multiple DEXes
        
        Args:
            token_in: Input token address
            token_out: Output token address
            amount_in: Amount in smallest unit
            
        Returns:
            Best quote with protocol information
        """
        quotes = []
        
        # Try V2 quote
        v2_quote = await self.get_v2_quote(token_in, token_out, amount_in)
        if v2_quote["success"]:
            quotes.append(v2_quote)
        
        # Try V3 with different fee tiers
        for fee in [500, 3000, 10000]:  # 0.05%, 0.3%, 1%
            v3_quote = await self.get_v3_quote(token_in, token_out, amount_in, fee)
            if v3_quote["success"]:
                quotes.append(v3_quote)
        
        # Find best quote (highest output)
        if quotes:
            best_quote = max(quotes, key=lambda q: q["amountOut"])
            return best_quote
        else:
            return {
                "protocol": "NONE",
                "amountIn": amount_in,
                "amountOut": 0,
                "price": 0,
                "fee": 0,
                "success": False,
                "error": "No valid quotes found"
            }
    
    async def build_slippage_curve(
        self,
        token_in: str,
        token_out: str,
        amounts: List[int]
    ) -> List[Dict]:
        """
        Build slippage curve by getting quotes for multiple amounts
        
        Args:
            token_in: Input token address
            token_out: Output token address
            amounts: List of amounts to quote
            
        Returns:
            List of quotes showing price impact at different sizes
        """
        curve = []
        
        for amount in amounts:
            quote = await self.get_best_quote(token_in, token_out, amount)
            
            # Calculate slippage vs smallest amount
            if curve and curve[0]["success"]:
                base_price = curve[0]["price"]
                slippage = abs(quote["price"] - base_price) / base_price * 100
                quote["slippage"] = slippage
            else:
                quote["slippage"] = 0
                
            curve.append(quote)
        
        return curve