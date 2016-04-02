#/bin/sh -f

# things to do for travis-ci in the before_install section

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then
  brew update
  brew outdated cmake || brew upgrade cmake
  brew outdated boost || brew upgrade boost
  brew install ragel
else
  sudo apt-get update -qq
  sudo apt-get install -y cmake ragel g++ g++-4.8

  sudo rm -f /usr/bin/g++
  sudo rm -f /usr/bin/gcc
  sudo rm -f /usr/bin/gcov
  sudo ln -s /usr/bin/g++-4.8 /usr/bin/g++
  sudo ln -s /usr/bin/gcc-4.8 /usr/bin/gcc
  sudo ln -s /usr/bin/gcov-4.8 /usr/bin/gcov

  wget http://downloads.sourceforge.net/project/boost/boost/1.60.0/boost_1_60_0.tar.gz -O /tmp/boost.tar.gz
  tar -xzf /tmp/boost.tar.gz
  pushd boost_1_60_0
  ./bootstrap.sh
  sudo ./b2 -q -d=0 install -j 2 --prefix=/usr link=static
  popd
fi