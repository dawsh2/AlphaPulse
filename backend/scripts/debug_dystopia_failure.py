#!/usr/bin/env python3
"""Debug why the Dystopia arbitrage execution failed"""

from web3 import Web3
import json

w3 = Web3(Web3.HTTPProvider('https://polygon.publicnode.com'))

# Transaction that failed
tx_hash = '0x51952ee20ad0e357c909f6f65bd5ad20ce5caed73e1718e9533e290ce648992c'

print("Analyzing failed transaction...")
print("="*60)

# Get transaction details
tx = w3.eth.get_transaction(tx_hash)
receipt = w3.eth.get_transaction_receipt(tx_hash)

print(f"Transaction: {tx_hash}")
print(f"Status: {'Success' if receipt['status'] == 1 else 'FAILED'}")
print(f"Gas Used: {receipt['gasUsed']:,} / {tx['gas']:,}")
print(f"From: {tx['from']}")
print(f"To: {tx['to']}")
print(f"Value: {Web3.from_wei(tx['value'], 'ether')} MATIC")

# Decode the function call
contract_addr = '0x2a36DED40Dc15935dd3fA31d035D2Ed880290e67'
function_selector = tx['input'][:10]
print(f"\nContract: {contract_addr}")
print(f"Function selector: {function_selector}")

# Try to get revert reason
print("\nAttempting to replay transaction to get revert reason...")

# Load contract ABI
contract_abi = """[{"inputs":[{"internalType":"uint256","name":"flashAmount","type":"uint256"}],"name":"executeArbitrage","outputs":[],"stateMutability":"nonpayable","type":"function"}]"""

contract = w3.eth.contract(address=Web3.to_checksum_address(contract_addr), abi=json.loads(contract_abi))

# Decode the input data
if function_selector == '0x5ea41926':  # executeArbitrage selector
    decoded = contract.decode_function_input(tx['input'])
    flash_amount = decoded[1]['flashAmount']
    print(f"Function: executeArbitrage")
    print(f"Flash Amount: {flash_amount / 10**6:.2f} USDC")

# Check current state of the pools
print("\n" + "="*60)
print("Current Pool States:")

# Buy pool (Dystopia)
buy_pool = '0x380615f37993b5a96adf3d443b6e0ac50a211998'
# Sell pool (QuickSwap)
sell_pool = '0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2'

pool_abi = json.loads('[{"inputs":[],"name":"getReserves","outputs":[{"name":"","type":"uint112"},{"name":"","type":"uint112"}],"type":"function"},{"inputs":[],"name":"token0","outputs":[{"name":"","type":"address"}],"type":"function"},{"inputs":[],"name":"token1","outputs":[{"name":"","type":"address"}],"type":"function"}]')

for name, addr in [("Buy Pool (Dystopia)", buy_pool), ("Sell Pool (QuickSwap)", sell_pool)]:
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(addr), abi=pool_abi)
        reserves = pool.functions.getReserves().call()
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        
        print(f"\n{name}:")
        print(f"  Address: {addr}")
        print(f"  Token0: {token0}")
        print(f"  Token1: {token1}")
        print(f"  Reserves: {reserves[0]/10**18:.2f} / {reserves[1]/10**6:.2f}")
        
        # Calculate price
        if token0.lower() == '0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270':  # WPOL
            price = reserves[1] / reserves[0] * 10**12
            print(f"  WPOL Price: ${price:.6f}")
    except Exception as e:
        print(f"\n{name}: Error - {e}")

# Try to simulate the transaction
print("\n" + "="*60)
print("Attempting to simulate the failed transaction...")

dystopia_router = '0xbE75Dd16D029c6B32B7aD57A0FD9C1c20Dd2862e'

# Check if Dystopia router has the expected functions
router_abi = json.loads("""[
    {"inputs":[{"internalType":"uint256","name":"amountIn","type":"uint256"},{"components":[{"internalType":"address","name":"from","type":"address"},{"internalType":"address","name":"to","type":"address"},{"internalType":"bool","name":"stable","type":"bool"}],"internalType":"struct IDystopiaRouter.route[]","name":"routes","type":"tuple[]"}],"name":"getAmountsOut","outputs":[{"internalType":"uint256[]","name":"amounts","type":"uint256[]"}],"stateMutability":"view","type":"function"}
]""")

try:
    router = w3.eth.contract(address=Web3.to_checksum_address(dystopia_router), abi=router_abi)
    
    # Test getAmountsOut
    routes = [(
        '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174',  # USDC_OLD
        '0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270',  # WPOL
        False  # not stable
    )]
    
    test_amount = 100 * 10**6  # 100 USDC
    amounts_out = router.functions.getAmountsOut(test_amount, routes).call()
    print(f"\nDystopia Router Test:")
    print(f"  Input: 100 USDC_OLD")
    print(f"  Expected output: {amounts_out[1]/10**18:.4f} WPOL")
    print(f"  Effective price: ${test_amount/amounts_out[1] * 10**12:.6f}")
except Exception as e:
    print(f"\nDystopia Router Test Failed: {e}")

# Check if the contract has enough approvals
print("\n" + "="*60)
print("Checking for common failure reasons:")

print("\n1. Pool Liquidity Changed:")
print("   The arbitrage opportunity may have been taken by MEV bots")

print("\n2. Dystopia Interface Mismatch:")
print("   The Dystopia router might use a different function signature")

print("\n3. Missing USDC Conversion:")
print("   Contract gets USDC.e but needs USDC_OLD to repay flash loan")

print("\n4. Slippage Protection:")
print("   Some pools may have built-in MEV protection")

# Try with standard V2 pools instead
print("\n" + "="*60)
print("Alternative: Use standard V2 pools only")

# Find V2 pools with good liquidity
v2_candidates = [
    ('0x6d9e8dbb2779853db00418d4dcf96f3987cfc9d2', 'QuickSwap WPOL/USDC.e'),
    ('0x6e7a5fafcec6bb1e78bae2a1f0b612012bf14827', 'Unknown V2'),
    ('0x29a92b95be45d5bdd638b749798f0fee107fdbc7', 'Unknown V2'),
]

print("\nScanning for V2 arbitrage opportunities...")
for addr, name in v2_candidates:
    try:
        pool = w3.eth.contract(address=Web3.to_checksum_address(addr), abi=pool_abi)
        reserves = pool.functions.getReserves().call()
        token0 = pool.functions.token0().call()
        token1 = pool.functions.token1().call()
        
        if token0.lower() == '0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270':  # WPOL
            price = reserves[1] / reserves[0] * 10**12
            usdc_type = 'USDC.e' if token1.lower() == '0x3c499c542cef5e3811e1192ce70d8cc03d5c3359' else 'USDC_OLD'
            print(f"{addr[:10]}... ({name}): WPOL/{usdc_type} @ ${price:.6f}")
    except:
        pass

print("\n" + "="*60)
print("RECOMMENDATION:")
print("The Dystopia pool likely uses a different swap interface than expected.")
print("Options:")
print("1. Use DirectPoolArbitrage with two standard V2 pools")
print("2. Research Dystopia's exact interface and update the contract")
print("3. Find alternative buy pools that use standard V2 interface")