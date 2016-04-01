#/bin/sh -f

# things to do for travis-ci in the before_install section

if ( test "`uname -s`" = "Darwin" )
then
  brew update
  brew install cmake boost
else
  sudo apt-get update -qq
  sudo apt-get install cmake boost
fi