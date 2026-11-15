#ifndef PILOT_RT_BRIDGE_H
#define PILOT_RT_BRIDGE_H

#include <dirent.h>
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <netdb.h>
#include <poll.h>
#include <signal.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static inline int64_t pilot_errno_map(int e) {
    switch (e) {
    case EPERM: return -1;
    case EACCES: return -2;
    case ENOENT: return -3;
    case EEXIST: return -4;
    case EINTR: return -5;
    case EIO: return -6;
    case EAGAIN: return -7;
#ifdef EINPROGRESS
    case EINPROGRESS: return -8;
#endif
#ifdef EALREADY
    case EALREADY: return -9;
#endif
    case ENOMEM: return -10;
    case EBADF: return -11;
    case EINVAL: return -12;
    case EISDIR: return -13;
    case ENOTDIR: return -14;
    case ENOTEMPTY: return -15;
    case EBUSY: return -16;
    case EXDEV: return -17;
    case ENOSPC: return -18;
    case EROFS: return -19;
    case EMFILE: return -21;
    case ENFILE: return -22;
    case ENAMETOOLONG: return -23;
    case ELOOP: return -24;
    case EFBIG: return -25;
    case ESPIPE: return -26;
    case EPIPE: return -27;
    case ESRCH: return -28;
    case ECHILD: return -29;
    case ETIMEDOUT: return -30;
    case ECONNREFUSED: return -31;
    case ECONNRESET: return -32;
    case ECONNABORTED: return -33;
    case ENOTCONN: return -34;
    case EISCONN: return -35;
    case EADDRINUSE: return -36;
    case EADDRNOTAVAIL: return -37;
    case ENETUNREACH: return -38;
    case EHOSTUNREACH: return -39;
    case EMSGSIZE: return -40;
    case EAFNOSUPPORT: return -41;
    case ENOTSUP: return -42;
    case ENOSYS: return -43;
    case ERANGE: return -44;
    default: return -51;
    }
}

static inline char *pilot_bridge_cstr(int64_t ptr, int64_t len) {
    size_t n = len > 0 ? (size_t)len : 0;
    char *s = (char *)malloc(n + 1);
    if (!s) return NULL;
    if (n > 0) memcpy(s, (const void *)(uintptr_t)ptr, n);
    s[n] = '\0';
    return s;
}

static inline int64_t pilot_bridge_mkdir_parents(const char *path, mode_t mode) {
    char *tmp = strdup(path);
    if (!tmp) return -10;
    size_t len = strlen(tmp);
    for (size_t i = 1; i < len; i++) {
        if (tmp[i] == '/') {
            tmp[i] = '\0';
            if (mkdir(tmp, mode) != 0 && errno != EEXIST) {
                int e = errno;
                free(tmp);
                return pilot_errno_map(e);
            }
            tmp[i] = '/';
        }
    }
    if (mkdir(tmp, mode) != 0 && errno != EEXIST) {
        int e = errno;
        free(tmp);
        return pilot_errno_map(e);
    }
    free(tmp);
    return 0;
}

static inline int64_t pilot_bridge_rm_rf(const char *path) {
    struct stat st;
    if (lstat(path, &st) != 0) return pilot_errno_map(errno);
    if (!S_ISDIR(st.st_mode)) {
        return unlink(path) == 0 ? 0 : pilot_errno_map(errno);
    }
    DIR *d = opendir(path);
    if (!d) return pilot_errno_map(errno);
    struct dirent *ent;
    while ((ent = readdir(d)) != NULL) {
        if (strcmp(ent->d_name, ".") == 0 || strcmp(ent->d_name, "..") == 0) continue;
        size_t plen = strlen(path), nlen = strlen(ent->d_name);
        char *sub = (char *)malloc(plen + nlen + 2);
        if (!sub) { closedir(d); return -10; }
        sprintf(sub, "%s/%s", path, ent->d_name);
        int64_t rc = pilot_bridge_rm_rf(sub);
        free(sub);
        if (rc < 0) { closedir(d); return rc; }
    }
    closedir(d);
    return rmdir(path) == 0 ? 0 : pilot_errno_map(errno);
}

static inline pilot_val pilot_str_copy_to_ptr(pilot_val s, pilot_val ptr, pilot_val cap) {
    size_t n = pilot_v_slen(s);
    size_t c = cap.as.i > 0 ? (size_t)cap.as.i : 0;
    if (n > c) n = c;
    if (n > 0 && ptr.as.u != 0) memcpy((void *)(uintptr_t)ptr.as.u, pilot_v_sbytes(&s), n);
    return pilot_v_int((int64_t)n);
}

static inline pilot_val pilot_str_from_ptr(pilot_val ptr, pilot_val len) {
    int64_t n = len.as.i;
    if (n <= 0) return pilot_v_str("", 0);
    return pilot_v_str((const char *)(uintptr_t)ptr.as.u, (size_t)n);
}

static inline pilot_val pilot_vm_thread_entry_ptr(void) { return pilot_v_int(0); }
static inline pilot_val pilot_vm_callback_dispatch_ptr(void) { return pilot_v_int(0); }

static inline pilot_val pilot_host_op(
    pilot_val v0, pilot_val v1, pilot_val v2, pilot_val v3,
    pilot_val v4, pilot_val v5, pilot_val v6
) {
    int64_t op = v0.as.i;
    int64_t a0 = v1.as.i, a1 = v2.as.i, a2 = v3.as.i;
    int64_t a3 = v4.as.i, a4 = v5.as.i, a5 = v6.as.i;

    switch (op) {
    case 0: { /* SlabAlloc */
        size_t sz = a0 > 0 ? (size_t)a0 : 1;
        void *p = calloc(1, sz);
        return pilot_v_int((int64_t)(uintptr_t)p);
    }
    case 1: { /* SlabRealloc */
        size_t nsz = a2 > 0 ? (size_t)a2 : 1;
        void *p = realloc((void *)(uintptr_t)a0, nsz);
        return pilot_v_int((int64_t)(uintptr_t)p);
    }
    case 2: { /* SlabFree */
        free((void *)(uintptr_t)a0);
        return pilot_v_int(0);
    }
    case 3: return pilot_v_int(-43); /* MemMap */
    case 4: memcpy((void *)(uintptr_t)a0, (const void *)(uintptr_t)a1, (size_t)a2); return pilot_v_int(0);
    case 5: memset((void *)(uintptr_t)a0, (int)a1, (size_t)a2); return pilot_v_int(0);
    case 6: {
        int c = memcmp((const void *)(uintptr_t)a0, (const void *)(uintptr_t)a1, (size_t)a2);
        return pilot_v_int(c < 0 ? -1 : (c > 0 ? 1 : 0));
    }
    case 7: { /* LoadInt */
        const uint8_t *p = (const uint8_t *)(uintptr_t)a0;
        int64_t val = 0;
        if (a1 == 8) val = a2 ? (int64_t)*(const int8_t *)p : (int64_t)*p;
        else if (a1 == 16) { int16_t s; uint16_t u; if (a2) { memcpy(&s, p, 2); val = s; } else { memcpy(&u, p, 2); val = u; } }
        else if (a1 == 32) { int32_t s; uint32_t u; if (a2) { memcpy(&s, p, 4); val = s; } else { memcpy(&u, p, 4); val = u; } }
        else { memcpy(&val, p, 8); }
        return pilot_v_int(val);
    }
    case 8: { /* StoreInt */
        uint8_t *p = (uint8_t *)(uintptr_t)a0;
        if (a2 == 8) { uint8_t b = (uint8_t)a1; memcpy(p, &b, 1); }
        else if (a2 == 16) { uint16_t s = (uint16_t)a1; memcpy(p, &s, 2); }
        else if (a2 == 32) { uint32_t w = (uint32_t)a1; memcpy(p, &w, 4); }
        else { int64_t q = a1; memcpy(p, &q, 8); }
        return pilot_v_int(0);
    }
    case 9: { /* LoadFloat */
        const uint8_t *p = (const uint8_t *)(uintptr_t)a0;
        if (a1 == 32) { float f; memcpy(&f, p, 4); return pilot_v_float((double)f); }
        double d; memcpy(&d, p, 8); return pilot_v_float(d);
    }
    case 10: { /* StoreFloat */
        uint8_t *p = (uint8_t *)(uintptr_t)a0;
        double d = (v2.tag == PILOT_V_FLOAT) ? v2.as.f : (double)v2.as.i;
        if (a2 == 32) { float f = (float)d; memcpy(p, &f, 4); }
        else { memcpy(p, &d, 8); }
        return pilot_v_int(0);
    }
    case 11: { /* AtomicInt */
        volatile int64_t *p = (volatile int64_t *)(uintptr_t)a1;
        if (a0 == 0) return pilot_v_int(__atomic_load_n(p, __ATOMIC_SEQ_CST));
        if (a0 == 1) { __atomic_store_n(p, a3, __ATOMIC_SEQ_CST); return pilot_v_int(0); }
        if (a0 == 3) return pilot_v_int(__atomic_fetch_add(p, a3, __ATOMIC_SEQ_CST));
        if (a0 == 9) {
            int64_t exp = a2;
            bool ok = __atomic_compare_exchange_n(p, &exp, a3, false, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
            return pilot_v_int(ok ? 1 : 0);
        }
        return pilot_v_int(-12);
    }
    case 14: { /* ThreadCtl */
        if (a0 == 2) {
            long n = sysconf(_SC_NPROCESSORS_ONLN);
            return pilot_v_int(n >= 1 ? (int64_t)n : 1);
        }
        return pilot_v_int(-43);
    }
    case 16: { /* FdOpen */
        char *p = pilot_bridge_cstr(a0, a1);
        if (!p) return pilot_v_int(-10);
        int flags = 0;
        bool rd = (a2 & 1) != 0, wr = (a2 & 2) != 0;
        if (rd && wr) flags |= O_RDWR;
        else if (wr) flags |= O_WRONLY;
        else flags |= O_RDONLY;
        if (a2 & 4) flags |= O_CREAT;
        if (a2 & 8) flags |= O_TRUNC;
        if (a2 & 16) flags |= O_APPEND;
        if (a2 & 32) flags |= O_EXCL;
        if (a2 & 64) flags |= O_NONBLOCK;
        int fd = open(p, flags, (mode_t)a3);
        int e = errno;
        free(p);
        return pilot_v_int(fd >= 0 ? (int64_t)fd : pilot_errno_map(e));
    }
    case 17: return pilot_v_int(close((int)a0) == 0 ? 0 : pilot_errno_map(errno));
    case 18: { ssize_t r = read((int)a0, (void *)(uintptr_t)a1, (size_t)a2); return pilot_v_int(r >= 0 ? (int64_t)r : pilot_errno_map(errno)); }
    case 19: { ssize_t w = write((int)a0, (const void *)(uintptr_t)a1, (size_t)a2); return pilot_v_int(w >= 0 ? (int64_t)w : pilot_errno_map(errno)); }
    case 20: { ssize_t r = pread((int)a0, (void *)(uintptr_t)a1, (size_t)a2, (off_t)a3); return pilot_v_int(r >= 0 ? (int64_t)r : pilot_errno_map(errno)); }
    case 21: { ssize_t w = pwrite((int)a0, (const void *)(uintptr_t)a1, (size_t)a2, (off_t)a3); return pilot_v_int(w >= 0 ? (int64_t)w : pilot_errno_map(errno)); }
    case 22: {
        if (a0 >= 0 && a0 <= 2) {
            off_t r = lseek((int)a1, (off_t)a2, a0 == 0 ? SEEK_SET : (a0 == 1 ? SEEK_CUR : SEEK_END));
            return pilot_v_int(r >= 0 ? (int64_t)r : pilot_errno_map(errno));
        }
        return pilot_v_int(fsync((int)a1) == 0 ? 0 : pilot_errno_map(errno));
    }
    case 23: { /* FdPipeDup */
        if (a0 == 0) {
            int fds[2];
            if (pipe(fds) != 0) return pilot_v_int(pilot_errno_map(errno));
            int64_t *out = (int64_t *)(uintptr_t)a1;
            out[0] = (int64_t)fds[0];
            out[1] = (int64_t)fds[1];
            return pilot_v_int(0);
        }
        return pilot_v_int(-12);
    }
    case 24: { /* IoPoll */
        typedef struct { int64_t fd; uint32_t ev; uint32_t rev; } os_pfd_t;
        os_pfd_t *in = (os_pfd_t *)(uintptr_t)a0;
        size_t n = (size_t)a1;
        struct pollfd *pfds = (struct pollfd *)calloc(n ? n : 1, sizeof(struct pollfd));
        for (size_t i = 0; i < n; i++) {
            pfds[i].fd = (int)in[i].fd;
            if (in[i].ev & 1) pfds[i].events |= POLLIN;
            if (in[i].ev & 2) pfds[i].events |= POLLOUT;
        }
        int r = poll(pfds, (nfds_t)n, (int)a2);
        int e = errno;
        if (r >= 0) {
            for (size_t i = 0; i < n; i++) {
                uint32_t rv = 0;
                if (pfds[i].revents & POLLIN) rv |= 1;
                if (pfds[i].revents & POLLOUT) rv |= 2;
                if (pfds[i].revents & POLLERR) rv |= 4;
                if (pfds[i].revents & POLLHUP) rv |= 8;
                if (pfds[i].revents & POLLNVAL) rv |= 16;
                in[i].rev = rv;
            }
        }
        free(pfds);
        return pilot_v_int(r >= 0 ? (int64_t)r : pilot_errno_map(e));
    }
    case 26: { /* FsStat */
        char *p = pilot_bridge_cstr(a1, a2);
        if (!p) return pilot_v_int(-10);
        struct stat st;
        int r = a0 ? stat(p, &st) : lstat(p, &st);
        int e = errno;
        free(p);
        if (r != 0) return pilot_v_int(pilot_errno_map(e));
        uint64_t *out = (uint64_t *)(uintptr_t)a3;
        uint64_t k = 0;
        if (S_ISREG(st.st_mode)) k = 1;
        else if (S_ISDIR(st.st_mode)) k = 2;
        else if (S_ISLNK(st.st_mode)) k = 3;
        out[0] = k;
        out[1] = (uint64_t)st.st_size;
        out[2] = (uint64_t)st.st_mode;
        out[3] = (uint64_t)st.st_mtime * 1000000000ULL;
        out[4] = (uint64_t)st.st_atime * 1000000000ULL;
        out[5] = (uint64_t)st.st_ctime * 1000000000ULL;
        out[6] = (uint64_t)st.st_dev;
        out[7] = (uint64_t)st.st_ino;
        out[8] = (uint64_t)st.st_nlink;
        return pilot_v_int(0);
    }
    case 27: { /* FsReadDir */
        char *p = pilot_bridge_cstr(a0, a1);
        if (!p) return pilot_v_int(-10);
        DIR *d = opendir(p);
        int e = errno;
        free(p);
        if (!d) return pilot_v_int(pilot_errno_map(e));
        uint8_t *dst = (uint8_t *)(uintptr_t)a2;
        int64_t cap = a3, pos = 0, cnt = 0;
        struct dirent *ent;
        while ((ent = readdir(d)) != NULL) {
            if (strcmp(ent->d_name, ".") == 0 || strcmp(ent->d_name, "..") == 0) continue;
            size_t nlen = strlen(ent->d_name);
            if (pos + 3 + (int64_t)nlen > cap) { closedir(d); return pilot_v_int(-44); }
            uint8_t kind = 0;
#ifdef DT_DIR
            if (ent->d_type == DT_DIR) kind = 2;
            else if (ent->d_type == DT_REG) kind = 1;
            else if (ent->d_type == DT_LNK) kind = 3;
#endif
            dst[pos] = kind;
            dst[pos + 1] = (uint8_t)(nlen & 0xff);
            dst[pos + 2] = (uint8_t)((nlen >> 8) & 0xff);
            memcpy(dst + pos + 3, ent->d_name, nlen);
            pos += 3 + (int64_t)nlen;
            cnt++;
        }
        closedir(d);
        if (a4 != 0) *(int64_t *)(uintptr_t)a4 = pos;
        return pilot_v_int(cnt);
    }
    case 28: { /* FsMkdir */
        char *p = pilot_bridge_cstr(a0, a1);
        if (!p) return pilot_v_int(-10);
        int64_t rc = a2 ? pilot_bridge_mkdir_parents(p, (mode_t)a3) : (mkdir(p, (mode_t)a3) == 0 ? 0 : pilot_errno_map(errno));
        free(p);
        return pilot_v_int(rc);
    }
    case 29: { /* FsRemove */
        char *p = pilot_bridge_cstr(a0, a1);
        if (!p) return pilot_v_int(-10);
        int64_t rc = a2 ? pilot_bridge_rm_rf(p) : (unlink(p) == 0 ? 0 : (errno == EISDIR || errno == EPERM ? (rmdir(p) == 0 ? 0 : pilot_errno_map(errno)) : pilot_errno_map(errno)));
        free(p);
        return pilot_v_int(rc);
    }
    case 30: { /* FsRenameLink */
        char *s = pilot_bridge_cstr(a1, a2), *d = pilot_bridge_cstr(a3, a4);
        int64_t rc = -12;
        if (s && d) {
            if (a0 == 0) rc = rename(s, d) == 0 ? 0 : pilot_errno_map(errno);
            else if (a0 == 1) rc = link(s, d) == 0 ? 0 : pilot_errno_map(errno);
            else if (a0 == 2) rc = symlink(s, d) == 0 ? 0 : pilot_errno_map(errno);
        }
        free(s); free(d);
        return pilot_v_int(rc);
    }
    case 32: { /* ProcSpawn */
        const uint64_t *sp = (const uint64_t *)(uintptr_t)a0;
        char *prog = pilot_bridge_cstr((int64_t)sp[0], (int64_t)sp[1]);
        size_t argc = (size_t)sp[3];
        const uint64_t *av = (const uint64_t *)(uintptr_t)sp[2];
        char **argv = (char **)calloc(argc + 2, sizeof(char *));
        argv[0] = strdup(prog);
        for (size_t i = 0; i < argc; i++) argv[i + 1] = pilot_bridge_cstr((int64_t)av[i * 2], (int64_t)av[i * 2 + 1]);
        char *cwd = sp[7] > 0 ? pilot_bridge_cstr((int64_t)sp[6], (int64_t)sp[7]) : NULL;
        int64_t in_fd = (int64_t)sp[8], out_fd = (int64_t)sp[9], err_fd = (int64_t)sp[10];
        pid_t pid = fork();
        if (pid == 0) {
            if (cwd && chdir(cwd) != 0) _exit(127);
            if (in_fd >= 0) dup2((int)in_fd, STDIN_FILENO);
            if (out_fd >= 0) dup2((int)out_fd, STDOUT_FILENO);
            if (err_fd >= 0) dup2((int)err_fd, STDERR_FILENO);
            execvp(prog, argv);
            _exit(127);
        }
        free(prog); free(cwd);
        for (size_t i = 0; i <= argc; i++) free(argv[i]);
        free(argv);
        if (pid < 0) return pilot_v_int(pilot_errno_map(errno));
        if (a1 != 0) *(int64_t *)(uintptr_t)a1 = (int64_t)pid;
        return pilot_v_int((int64_t)pid);
    }
    case 33: { /* ProcWait */
        int st = 0;
        pid_t r = waitpid((pid_t)a0, &st, a1 ? WNOHANG : 0);
        if (r == 0) return pilot_v_int(0);
        if (r < 0) return pilot_v_int(pilot_errno_map(errno));
        int64_t code = WIFEXITED(st) ? (int64_t)WEXITSTATUS(st) : -1;
        if (a2 != 0) *(int64_t *)(uintptr_t)a2 = code;
        return pilot_v_int(1);
    }
    case 34: { /* ProcSignal */
        if (a0 == 2) return pilot_v_int((int64_t)getpid());
        return pilot_v_int(kill((pid_t)a1, (int)a2) == 0 ? 0 : pilot_errno_map(errno));
    }
    case 35: { /* EnvCtl */
        if (a0 == 0) {
            char *k = pilot_bridge_cstr(a1, a2);
            const char *v = k ? getenv(k) : NULL;
            free(k);
            if (!v) return pilot_v_int(-3);
            size_t vl = strlen(v);
            if (a5 != 0) *(int64_t *)(uintptr_t)a5 = (int64_t)vl;
            if ((int64_t)vl > a4) return pilot_v_int(-44);
            memcpy((void *)(uintptr_t)a3, v, vl);
            return pilot_v_int(0);
        }
        if (a0 == 2) {
            int64_t total = 0;
            for (int i = 1; i < pilot_argc; i++) total += 4 + (int64_t)strlen(pilot_argv[i]);
            if (a5 != 0) *(int64_t *)(uintptr_t)a5 = total;
            if (total > a4) return pilot_v_int(-44);
            uint8_t *dst = (uint8_t *)(uintptr_t)a3;
            int64_t off = 0;
            for (int i = 1; i < pilot_argc; i++) {
                uint32_t l = (uint32_t)strlen(pilot_argv[i]);
                memcpy(dst + off, &l, 4);
                memcpy(dst + off + 4, pilot_argv[i], l);
                off += 4 + (int64_t)l;
            }
            return pilot_v_int(0);
        }
        if (a0 == 3) {
            ssize_t r = readlink("/proc/self/exe", (char *)(uintptr_t)a3, (size_t)a4);
            if (r < 0) return pilot_v_int(pilot_errno_map(errno));
            if (a5 != 0) *(int64_t *)(uintptr_t)a5 = (int64_t)r;
            return pilot_v_int(0);
        }
        return pilot_v_int(-12);
    }
    case 36: { /* ClockNowNs */
        struct timespec ts;
        clock_gettime(a0 == 0 ? CLOCK_REALTIME : CLOCK_MONOTONIC, &ts);
        return pilot_v_int((int64_t)ts.tv_sec * 1000000000LL + (int64_t)ts.tv_nsec);
    }
    case 37: { /* SleepNs */
        struct timespec req = { (time_t)(a0 / 1000000000LL), (long)(a0 % 1000000000LL) };
        nanosleep(&req, NULL);
        return pilot_v_int(0);
    }
    case 38: { /* OsRandom */
        int fd = open("/dev/urandom", O_RDONLY);
        if (fd < 0) return pilot_v_int(pilot_errno_map(errno));
        /* Frozen ABI (PROP-002a §3): returns the number of bytes written. */
        size_t done = 0;
        while (done < (size_t)a1) {
            ssize_t r = read(fd, (uint8_t *)(uintptr_t)a0 + done, (size_t)a1 - done);
            if (r < 0 && errno == EINTR) continue;
            if (r <= 0) { int e = errno; close(fd); return pilot_v_int(r < 0 ? pilot_errno_map(e) : -51); }
            done += (size_t)r;
        }
        close(fd);
        return pilot_v_int((int64_t)done);
    }
    case 39: { /* NetSocket */
        int dom = a0 == 2 ? AF_INET6 : AF_INET;
        int ty = a1 == 2 ? SOCK_DGRAM : SOCK_STREAM;
        int fd = socket(dom, ty, (int)a2);
        return pilot_v_int(fd >= 0 ? (int64_t)fd : pilot_errno_map(errno));
    }
    case 40: { /* NetConnectBind */
        const uint8_t *ab = (const uint8_t *)(uintptr_t)a2;
        uint32_t fam; uint16_t port; uint16_t alen;
        memcpy(&fam, ab, 4); memcpy(&port, ab + 4, 2); memcpy(&alen, ab + 6, 2);
        struct sockaddr_storage ss; memset(&ss, 0, sizeof ss);
        socklen_t sl = 0;
        if (fam == 1) {
            struct sockaddr_in *s4 = (struct sockaddr_in *)&ss;
            s4->sin_family = AF_INET;
            s4->sin_port = htons(port);
            memcpy(&s4->sin_addr, ab + 8, 4);
            sl = sizeof(struct sockaddr_in);
        } else {
            struct sockaddr_in6 *s6 = (struct sockaddr_in6 *)&ss;
            s6->sin6_family = AF_INET6;
            s6->sin6_port = htons(port);
            memcpy(&s6->sin6_addr, ab + 8, 16);
            sl = sizeof(struct sockaddr_in6);
        }
        int r = a0 == 0 ? connect((int)a1, (struct sockaddr *)&ss, sl) : bind((int)a1, (struct sockaddr *)&ss, sl);
        return pilot_v_int(r == 0 ? 0 : pilot_errno_map(errno));
    }
    case 41: { /* NetIo */
        ssize_t r = a0 == 0 ? send((int)a1, (const void *)(uintptr_t)a2, (size_t)a3, (int)a4) : recv((int)a1, (void *)(uintptr_t)a2, (size_t)a3, (int)a4);
        return pilot_v_int(r >= 0 ? (int64_t)r : pilot_errno_map(errno));
    }
    case 42: { /* NetResolve */
        char *h = pilot_bridge_cstr(a0, a1);
        char pbuf[16]; snprintf(pbuf, sizeof pbuf, "%lld", (long long)a2);
        struct addrinfo hints, *res = NULL; memset(&hints, 0, sizeof hints);
        hints.ai_socktype = SOCK_STREAM;
        int rc = getaddrinfo(h, pbuf, &hints, &res);
        free(h);
        if (rc != 0 || !res) return pilot_v_int(-39);
        uint8_t *out = (uint8_t *)(uintptr_t)a3;
        memset(out, 0, 144);
        if (res->ai_family == AF_INET) {
            struct sockaddr_in *s4 = (struct sockaddr_in *)res->ai_addr;
            uint32_t fam = 1; uint16_t port = ntohs(s4->sin_port); uint16_t al = 4;
            memcpy(out, &fam, 4); memcpy(out + 4, &port, 2); memcpy(out + 6, &al, 2);
            memcpy(out + 8, &s4->sin_addr, 4);
        } else {
            struct sockaddr_in6 *s6 = (struct sockaddr_in6 *)res->ai_addr;
            uint32_t fam = 2; uint16_t port = ntohs(s6->sin6_port); uint16_t al = 16;
            memcpy(out, &fam, 4); memcpy(out + 4, &port, 2); memcpy(out + 6, &al, 2);
            memcpy(out + 8, &s6->sin6_addr, 16);
        }
        freeaddrinfo(res);
        if (a5 != 0) *(int64_t *)(uintptr_t)a5 = 144;
        return pilot_v_int(1);
    }
    case 43: { char *p = a1 > 0 ? pilot_bridge_cstr(a0, a1) : NULL; void *h = dlopen(p, RTLD_LAZY); free(p); return pilot_v_int(h ? (int64_t)(uintptr_t)h : -3); }
    case 44: { char *s = pilot_bridge_cstr(a1, a2); void *f = dlsym((void *)(uintptr_t)a0, s); free(s); if (!f) return pilot_v_int(-3); if (a3) *(int64_t *)(uintptr_t)a3 = (int64_t)(uintptr_t)f; return pilot_v_int(0); }
    case 45: return pilot_v_int(dlclose((void *)(uintptr_t)a0) == 0 ? 0 : -12);
    case 46: return pilot_v_int(-43); /* FfiCall: arch_kind check / seam */
    case 49: { char *m = pilot_bridge_cstr(a0, a1); pilot_trap(m ? m : "fault"); return pilot_v_int(0); }
    case 50: {
        char *id = pilot_bridge_cstr(a0, a1), *nt = pilot_bridge_cstr(a2, a3);
        fprintf(stderr, "seam: %s: %s\n", id ? id : "", nt ? nt : "");
        free(id); free(nt); exit(1);
    }
    default: pilot_trap("seam: unimplemented host_op in C11 runtime"); return pilot_v_int(-43);
    }
}

#endif /* PILOT_RT_BRIDGE_H */
