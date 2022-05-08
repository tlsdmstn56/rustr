#!/bin/bash
target=${1:-debug}
script_home=$(cd $(dirname ${BASH_SOURCE[0]}); pwd)
rustr_home=$(cd $script_home/..; pwd)
cd $rustr_home/target/${target}
R_binary=$(pwd)/rustr
R_binary=${R_binary//\//\\\/}
cd $(find . -name R-4.2.0)/bin
r_wrapper_script=$(sed "s/^R_binary=.*$/R_binary=${R_binary}/" ./R)
eval "$r_wrapper_script[@]"