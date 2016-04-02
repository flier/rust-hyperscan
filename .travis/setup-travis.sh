#/bin/sh -f
set -e

# things to do for travis-ci in the before_install section

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then
  brew update
  brew outdated cmake || brew upgrade cmake
  brew outdated boost || brew upgrade boost
  brew install ragel
else
  if [ ! -f "$HOME/boost/include/boost/config.hpp" ]; then
	wget http://downloads.sourceforge.net/project/boost/boost/1.60.0/boost_1_60_0.tar.gz -O /tmp/boost.tar.gz
	tar -xzf /tmp/boost.tar.gz
    cd boost_1_60_0 
    ./bootstrap.sh 
    ./b2 -q -d=0 install -j 2 --prefix=$HOME/boost link=static
  else
  	echo 'Using cached boost directory.';
  fi
fi