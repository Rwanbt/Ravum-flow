#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/*
 * Crash injection for the *release* binary.
 *
 * The product's own hook (`FLOW_TEST_CRASH`) is compiled out of release builds,
 * which is the right behaviour and the reason the crash windows of a shipped
 * binary were only ever proved in debug builds. This shim gives the real binary
 * deterministic crash points without touching it: it counts `rename` and `fsync`
 * calls and kills the process at the Nth one with _exit(137), which is
 * indistinguishable from an external kill.
 *
 *   FLOW_CRASH_ON=rename|fsync   FLOW_CRASH_AT=<n>
 *
 * No test may depend on a particular ordering of internal writes, so each case
 * states the call it kills and the script records what happened.
 */

static int (*real_rename)(const char *, const char *);
static int (*real_fsync)(int);
static long seen;
static long target = -1;
static const char *wanted = "rename";

static void load(void) {
    if (!real_rename) real_rename = (int (*)(const char *, const char *))dlsym(RTLD_NEXT, "rename");
    if (!real_fsync) real_fsync = (int (*)(int))dlsym(RTLD_NEXT, "fsync");
    if (target < 0) {
        const char *at = getenv("FLOW_CRASH_AT");
        const char *on = getenv("FLOW_CRASH_ON");
        target = at ? atol(at) : 0;
        if (on && *on) wanted = on;
    }
}

static void maybe_die(const char *op) {
    load();
    if (target <= 0) return;
    if (strcmp(op, wanted) != 0) return;
    if (++seen != target) return;
    _exit(137);
}

int rename(const char *from, const char *to) {
    maybe_die("rename");
    load();
    if (!real_rename) return -1;
    return real_rename(from, to);
}

int fsync(int fd) {
    maybe_die("fsync");
    load();
    if (!real_fsync) return -1;
    return real_fsync(fd);
}
