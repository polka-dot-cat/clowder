#define _GNU_SOURCE
#include "CClowder.h"
#include <errno.h>
#include <fcntl.h>
#include <pthread.h>
#include <signal.h>
#include <spawn.h>
#include <stdatomic.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

// The handler only writes a lock-free atomic. Cleanup happens outside it.
_Static_assert(ATOMIC_INT_LOCK_FREE == 2, "signal flag must be lock-free");
static atomic_int cancelled = 0;
static void on_signal(int signal) { atomic_store(&cancelled, signal); }
int clowder_cancelled(void) { return atomic_load(&cancelled); }
int clowder_install_signals(void) {
    struct sigaction action = {0};
    action.sa_handler = on_signal;
    sigemptyset(&action.sa_mask);
    for (int i = 0; i < 3; i++) {
        int signals[] = {SIGINT, SIGTERM, SIGHUP};
        if (sigaction(signals[i], &action, NULL)) return errno;
    }
    return 0;
}
static void pause_ms(long milliseconds) {
    struct timespec time = {milliseconds / 1000, milliseconds % 1000 * 1000000};
    while (nanosleep(&time, &time) && errno == EINTR) {}
}
static int foreground(int tty, pid_t group) {
    sigset_t mask, previous;
    sigemptyset(&mask);
    sigaddset(&mask, SIGTTOU);
    int result = pthread_sigmask(SIG_BLOCK, &mask, &previous);
    if (result) return result;
    result = tcsetpgrp(tty, group);
    pthread_sigmask(SIG_SETMASK, &previous, NULL);
    return result;
}
int clowder_run(const char *program, char *const argv[], char *const envp[],
                const char *directory, int input, int output, int error,
                int interactive, int *exit_status) {
    if (clowder_cancelled()) { *exit_status = 128 + clowder_cancelled(); return 0; }
    posix_spawn_file_actions_t files;
    posix_spawnattr_t attributes;
    int result = posix_spawn_file_actions_init(&files);
    if (result) return result;
    result = posix_spawnattr_init(&attributes);
    if (result) { posix_spawn_file_actions_destroy(&files); return result; }
    result = posix_spawn_file_actions_addchdir_np(&files, directory);
    if (!result && input >= 0) result = posix_spawn_file_actions_adddup2(&files, input, STDIN_FILENO);
    if (!result && output >= 0) result = posix_spawn_file_actions_adddup2(&files, output, STDOUT_FILENO);
    if (!result && error >= 0) result = posix_spawn_file_actions_adddup2(&files, error, STDERR_FILENO);
    sigset_t defaults, mask;
    sigemptyset(&mask);
    sigemptyset(&defaults);
    int signals[] = {SIGINT, SIGTERM, SIGHUP, SIGPIPE, SIGTSTP, SIGTTIN, SIGTTOU};
    for (unsigned i = 0; i < sizeof(signals) / sizeof(signals[0]); i++) sigaddset(&defaults, signals[i]);
    posix_spawnattr_setsigdefault(&attributes, &defaults);
    posix_spawnattr_setsigmask(&attributes, &mask);
    posix_spawnattr_setpgroup(&attributes, 0);
    posix_spawnattr_setflags(&attributes, POSIX_SPAWN_SETPGROUP | POSIX_SPAWN_SETSIGDEF | POSIX_SPAWN_SETSIGMASK);
    pid_t child = -1;
    if (!result) result = posix_spawnp(&child, program, &files, &attributes, argv, envp);
    posix_spawn_file_actions_destroy(&files);
    posix_spawnattr_destroy(&attributes);
    if (result) return result;
    int tty = -1;
    pid_t parent = getpgrp();
    if (interactive && isatty(STDIN_FILENO)) {
        tty = open("/dev/tty", O_RDONLY | O_CLOEXEC);
        if (tty >= 0 && (tcgetpgrp(tty) != parent || foreground(tty, child))) {
            close(tty); tty = -1;
        }
        if (tty >= 0) kill(-child, SIGCONT);
    }
    int status = 0;
    for (;;) {
        int signal = clowder_cancelled();
        if (signal) {
            kill(-child, signal);
            pause_ms(250); // Do not reap/reuse the group leader during the grace period.
            kill(-child, SIGKILL);
            while (waitpid(child, &status, 0) < 0 && errno == EINTR) {}
            *exit_status = 128 + signal;
            break;
        }
        // Observe without reaping: cancellation must finish signalling the
        // group before its leader's PID can be reused by an unrelated process.
        siginfo_t info = {0};
        int waited = waitid(P_PID, (id_t)child, &info,
                            WEXITED | WNOHANG | WNOWAIT | (tty >= 0 ? WSTOPPED : 0));
        if (waited < 0 && errno != EINTR) { result = errno; break; }
        if (waited == 0 && info.si_pid == child) {
            if (info.si_code == CLD_STOPPED) {
                while (waitpid(child, &status, WUNTRACED) < 0 && errno == EINTR) {}
                foreground(tty, parent);
                kill(getpid(), SIGSTOP);
                foreground(tty, child);
                kill(-child, SIGCONT);
                continue;
            }
            *exit_status = info.si_code == CLD_EXITED ? info.si_status : 128 + info.si_status;
            if (info.si_code != CLD_EXITED && (info.si_status == SIGINT || info.si_status == SIGTERM || info.si_status == SIGHUP)) {
                atomic_store(&cancelled, info.si_status);
                kill(-child, info.si_status);
                pause_ms(250);
                kill(-child, SIGKILL);
            }
            while (waitpid(child, &status, 0) < 0 && errno == EINTR) {}
            break;
        }
        pause_ms(2);
    }
    if (result) { kill(-child, SIGKILL); while (waitpid(child, &status, 0) < 0 && errno == EINTR) {} }
    if (tty >= 0) { foreground(tty, parent); close(tty); }
    return result;
}
