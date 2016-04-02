#/bin/sh -f
set -e

# things to do for travis-ci in the before_install section

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then
  brew update
  brew outdated cmake || brew upgrade cmake
  brew outdated boost || brew upgrade boost
  brew install ragel
else
  if [ ! -f "$HOME/boost_1_60_0/b2" ]; then
	wget http://downloads.sourceforge.net/project/boost/boost/1.60.0/boost_1_60_0.tar.gz -O /tmp/boost.tar.gz
	cd $HOME
	tar -xzf /tmp/boost.tar.gz
  else
  	echo 'Using cached boost directory.';
  fi

  cd $HOME/boost_1_60_0 
  ./bootstrap.sh 
  ./b2 -q -d=0 install -j 2 --prefix=/usr link=static
fi