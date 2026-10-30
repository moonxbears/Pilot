from setuptools import setup, find_packages

setup(
    name="pilot-kernel",
    version="1.1.51",
    description="Interactive Jupyter Kernel for the Pilot Programming Language",
    author="Pilot Language Contributors",
    packages=find_packages(),
    install_requires=[
        "ipykernel",
    ],
    entry_points={
        "console_scripts": [
            "pilot-kernel=pilot_kernel.__main__:main",
        ],
    },
    classifiers=[
        "Framework :: IPython",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Topic :: Software Development :: Interpreters",
    ],
)
