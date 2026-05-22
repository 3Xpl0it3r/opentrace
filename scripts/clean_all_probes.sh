# 用来手动调试的完清理
sudo bash -c '
    TR=/sys/kernel/debug/tracing
    # 停采集
    echo 0 > $TR/tracing_on

    # 关掉所有 events（tracepoint 和 kprobe events 都会被关）
    echo 0 > $TR/events/enable

    # 清空 kprobe events 定义
    echo > $TR/kprobe_events
    echo > $TR/uprobe_events 2>/dev/null || true

    # 重置 function tracer
    echo nop > $TR/current_tracer
    echo > $TR/set_ftrace_filter
    echo > $TR/set_ftrace_notrace
    echo > $TR/set_event_pid 2>/dev/null || true

    # 清空缓冲区
    echo > $TR/trace

    echo "==== cleanup done ===="
  '
