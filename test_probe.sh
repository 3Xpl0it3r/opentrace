type=$1
probe=$2
if [ "${type}" = "0" ]; then
	bpftrace -e "kprobe:${probe} { printf(\"kprobe:${probe} called\n\"); }"
else
	bpftrace -e "tracepoint:net:${probe} { printf(\"tracepoint:net:${probe} called\n\"); }"
fi
