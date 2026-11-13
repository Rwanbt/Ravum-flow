/* Wave 9 — durability shim.
 *
 * `state::write_atomic_bytes` flushes the containing directory after the rename
 * (`#[cfg(not(windows))]`) and turns a failure into an error. That rule is pinned
 * as a predicate by a unit test, but the *behaviour* — "a directory fsync is
 * attempted, and its failure is surfaced instead of swallowed" — cannot be
 * observed in-process without injecting a fault.
 *
 * This preloadable object answers both questions from outside the product:
 *   - it appends `dir-fsync fd=<n>` to $FLOW_FSYNC_LOG every time a directory
 *     descriptor is fsynced, which proves the call happens;
 *   - with FLOW_FSYNC_FAIL=1 it returns -1/EIO for those calls, which proves the
 *     error reaches the caller instead of being ignored.
 *
 * It is an experiment, never a gate: the product is not modified, and the shim is
 * only loaded into the process under test.
 */
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/stat.h>
#include <unistd.h>

static int (*real_fsync)(int);

int fsync(int fd)
{
    if (!real_fsync) {
        real_fsync = dlsym(RTLD_NEXT, "fsync");
        if (!real_fsync) {
            errno = ENOSYS;
            return -1;
        }
    }
    struct stat st;
    if (fstat(fd, &st) == 0 && S_ISDIR(st.st_mode)) {
        const char *log = getenv("FLOW_FSYNC_LOG");
        if (log && *log) {
            FILE *handle = fopen(log, "a");
            if (handle) {
                fprintf(handle, "dir-fsync fd=%d\n", fd);
                fclose(handle);
            }
        }
        const char *fail = getenv("FLOW_FSYNC_FAIL");
        if (fail && fail[0] == '1') {
            errno = EIO;
            return -1;
        }
    }
    return real_fsync(fd);
}
