#!/usr/bin/env python3
"""
AlphaPulse Rust Python Bindings Setup

This package provides ultra-low latency access to AlphaPulse's Rust shared memory infrastructure
from Python, enabling sub-10μs market data access for research and trading strategies.
"""

from setuptools import setup
from pyo3_setuptools_rust import Rust, RustExtension

setup(
    name="alphapulse-rust",
    version="0.1.0",
    author="AlphaPulse Team",
    author_email="dev@alphapulse.com",
    description="Ultra-low latency market data access for Python",
    long_description=open("README.md").read(),
    long_description_content_type="text/markdown",
    url="https://github.com/alphapulse/rust-services",
    rust_extensions=[
        RustExtension(
            "alphapulse_rust.alphapulse_rust",
            path="Cargo.toml",
            binding=Rust.PyO3,
            debug=False,
        )
    ],
    packages=["alphapulse_rust"],
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Financial and Insurance Industry",
        "License :: OSI Approved :: MIT License",
        "Operating System :: POSIX :: Linux",
        "Operating System :: MacOS",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Rust",
        "Topic :: Office/Business :: Financial",
        "Topic :: Scientific/Engineering",
    ],
    python_requires=">=3.8",
    install_requires=[
        "numpy>=1.20.0",
        "pandas>=1.3.0",
        "asyncio",
    ],
    extras_require={
        "dev": [
            "pytest>=6.0",
            "pytest-asyncio",
            "pytest-benchmark",
            "black",
            "mypy",
        ],
        "jupyter": [
            "jupyter",
            "matplotlib",
            "plotly",
            "ipywidgets",
        ],
        "analysis": [
            "scipy",
            "scikit-learn",
            "ta-lib",
        ],
    },
    zip_safe=False,
)