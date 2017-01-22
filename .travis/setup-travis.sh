#/bin/sh -f
set -e

# things to do for travis-ci in the before_install section

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then
	brew update
	brew outdated cmake || brew upgrade cmake
	# brew outdated boost || brew upgrade boost
	brew tap homebrew/versions
	brew install boost160 ragel tree llvm
else
	mkdir $HOME/bin

	ln -s /usr/bin/g++-4.8 $HOME/bin/g++
	ln -s /usr/bin/gcc-4.8 $HOME/bin/gcc
	ln -s /usr/bin/gcov-4.8 $HOME/bin/gcov

    export PATH=$HOME/bin:$PATH

	if [ ! -f "$BOOST_ROOT/lib/libboost_graph.a" ]; then
		wget http://downloads.sourceforge.net/project/boost/boost/1.$BOOST_VERSION_MINOR.0/boost_1_$BOOST_VERSION_MINOR\_0.tar.gz -O /tmp/boost.tar.gz
		tar -xzf /tmp/boost.tar.gz
		cd boost_1_$BOOST_VERSION_MINOR\_0
		./bootstrap.sh
		./b2 -q -d=0 install -j 2 --prefix=$BOOST_ROOT link=static
	else
  		echo 'Using cached boost v1.$BOOST_VERSION_MINOR_0 @ $BOOST_ROOT.';
  	fi

	function llvm_version_triple() {
	    if [ "$1" == "3.8" ]; then
	        echo "3.8.0"
	    elif [ "$1" == "3.9" ]; then
	        echo "3.9.0"
	    fi
	}

	function llvm_download() {
	    export LLVM_VERSION_TRIPLE=`llvm_version_triple ${LLVM_VERSION}`
	    export LLVM=clang+llvm-${LLVM_VERSION_TRIPLE}-x86_64-$1

	    echo 'Downloading llvm ${LLVM_VERSION} ...'

	    wget http://llvm.org/releases/${LLVM_VERSION_TRIPLE}/${LLVM}.tar.xz
	    mkdir llvm-$LLVM_VERSION
	    tar -xf ${LLVM}.tar.xz -C llvm-$LLVM_VERSION --strip-components=1

	    export LLVM_CONFIG_PATH=`pwd`/llvm-$LLVM_VERSION/bin/llvm-config
	}

	llvm_download linux-gnu-ubuntu-14.04
fi
