from setuptools import setup, find_packages

setup(
    name="alphapulse-adapters",
    version="0.1.0",
    description="AlphaPulse adapters for NautilusTrader",
    packages=find_packages(),
    install_requires=[
        "nautilus-trader>=1.0.0",
        "aiohttp>=3.8.0",
        "websockets>=10.0",
        "msgspec>=0.15.0",
    ],
    python_requires=">=3.10",
)