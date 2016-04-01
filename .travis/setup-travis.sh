#/bin/sh -f

# things to do for travis-ci in the before_install section

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then
  brew update
  brew outdated cmake || brew upgrade cmake
  brew outdated boost || brew upgrade boost
else
  sudo apt-get update -qq
  sudo apt-get install cmake 

  wget http://downloads.sourceforge.net/project/boost/boost/1.60.0/boost_1_60_0.tar.gz -O /tmp/boost.tar.gz
  tar -xzf /tmp/boost.tar.gz
  pushd boost_1_60_0
  ./bootstrap.sh
  sudo ./b2 install -j 2 --prefix=/usr link=static
  popd
fi