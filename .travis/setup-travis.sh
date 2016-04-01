#/bin/sh -f

# things to do for travis-ci in the before_install section

if [[ $TRAVIS_OS_NAME == 'osx' ]]; then
  brew update
  brew install cmake boost
else
  sudo apt-get update -qq
  sudo apt-get install cmake libboost-dev
fi