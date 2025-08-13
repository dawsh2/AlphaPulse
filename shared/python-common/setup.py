"""
Setup for shared Python library
"""

from setuptools import setup, find_packages

setup(
    name="alphapulse-shared",
    version="0.1.0",
    description="Shared Python utilities for AlphaPulse",
    packages=find_packages(),
    python_requires=">=3.8",
    install_requires=[
        # Add any shared dependencies here
    ],
)