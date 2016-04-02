#/bin/sh -f
set -e

wget https://github.com/01org/hyperscan/archive/v4.1.0.tar.gz -O /tmp/hyperscan.tar.gz
tar -xvf /tmp/hyperscan.tar.gz
cd hyperscan-4.1.0
cmake . -DCMAKE_INSTALL_PREFIX=$HOME/hyperscan -DCMAKE_POSITION_INDEPENDENT_CODE=on -DBOOST_ROOT=$HOME/boost
make
make install

