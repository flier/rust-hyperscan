#/bin/sh -f
set -e

# things to do for travis-ci in the before_install section

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then
  brew update
  brew outdated cmake || brew upgrade cmake
  brew outdated boost || brew upgrade boost
  brew install ragel
else
  apt-get update -qq
  apt-get install -y cmake ragel g++ g++-4.8

  rm -f /usr/bin/g++
  rm -f /usr/bin/gcc
  rm -f /usr/bin/gcov
  ln -s /usr/bin/g++-4.8 /usr/bin/g++
  ln -s /usr/bin/gcc-4.8 /usr/bin/gcc
  ln -s /usr/bin/gcov-4.8 /usr/bin/gcov

  if [ ! -d "$HOME/boost_1_60_0" ]; then
	wget http://downloads.sourceforge.net/project/boost/boost/1.60.0/boost_1_60_0.tar.gz -O /tmp/boost.tar.gz
	tar -xzf /tmp/boost.tar.gz
  else
  	echo 'Using cached boost directory.';
  fi

  cd boost_1_60_0 
  ./bootstrap.sh 
  ./b2 -q -d=0 install -j 2 --prefix=/usr link=static
fi