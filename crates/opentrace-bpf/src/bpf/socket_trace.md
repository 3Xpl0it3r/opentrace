```c
// sys_read

ssize_t ksys_read(unsigned int fd, char __user *buf, size_t count)
    -> ssize_t vfs_read(struct file *file, char __user *buf, size_t count, loff_t *pos)
        -> rw_verify_area(READ, file, pos, count)
        -> new_sync_read(file, buf, count, pos)
            -> static ssize_t sock_read_iter(struct kiocb *iocb, struct iov_iter *to)
                -> int sock_recvmsg(struct socket *sock, struct msghdr *msg, int flags)
                    -> int security_socket_recvmsg(struct socket *sock, struct msghdr *msg,

// sys_recv | sys_recvfrom

int __sys_recvfrom(int fd, void __user *ubuf, size_t size, unsigned int flags,
    -> static struct socket *sockfd_lookup_light(int fd, int *err, int *fput_needed)
        -> int sock_recvmsg(struct socket *sock, struct msghdr *msg, int flags)
        ....


// sys_recvmsg
long __sys_recvmsg(int fd, struct user_msghdr __user *msg, unsigned int flags,
    -> static struct socket *sockfd_lookup_light(int fd, int *err, int *fput_needed)
    -> static int ____sys_recvmsg(struct socket *sock, struct msghdr *msg_sys,
        // 在sys_recvmsg的时候nosec被设置为0， 所以会走sock_recvmsg分支
        -> int sock_recvmsg(struct socket *sock, struct msghdr *msg, int flags)
            -> int security_socket_recvmsg(struct socket *sock, struct msghdr *msg,




// sys_recvmmsg
int __sys_recvmmsg(int fd, struct mmsghdr __user *mmsg,
    -> static int do_recvmmsg(int fd, struct mmsghdr __user *mmsg,
        -> static struct socket *sockfd_lookup_light(int fd, int *err, int *fput_needed)
            // ___sys_recvmsg里面第一个数据走secutiry_socket_recvmsg
            -> while do static int ___sys_recvmsg(struct socket *sock, struct user_msghdr __user *msg,




// sys_readv
static ssize_t do_readv(unsigned long fd, const struct iovec __user *vec,
    -> static ssize_t vfs_readv(struct file *file, const struct iovec __user *vec,
        -> static ssize_t do_iter_read(struct file *file, struct iov_iter *iter,
            -> static ssize_t do_iter_readv_writev(struct file *filp, struct iov_iter *iter,
                -> static inline ssize_t call_read_iter(struct file *file, struct kiocb *kio,
                    // return file->f_op->read_iter(kio, iter);
                    -> static ssize_t sock_read_iter(struct kiocb *iocb, struct iov_iter *to)
                        -> int sock_recvmsg(struct socket *sock, struct msghdr *msg, int flags)
                            -> int security_socket_recvmsg(struct socket *sock, struct msghdr *msg,
```

