tpname=$1
tp=$(echo ${tpname}|sed 's/\//:/g')

sudo bpftrace -lv "${tp}"
