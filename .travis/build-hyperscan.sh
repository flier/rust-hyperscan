#/bin/sh -f
set -e

if [ ! -d "$HOME/hyperscan-4.1.0" ]; then
	wget https://github.com/01org/hyperscan/archive/v4.1.0.tar.gz -O /tmp/hyperscan.tar.gz
	cd $HOME
	tar -xvf /tmp/hyperscan.tar.gz
else
	echo 'Using cached hyperscan directory.';
fi

cd $HOME/hyperscan-4.1.0
cmake . -DCMAKE_POSITION_INDEPENDENT_CODE=on
make
make install