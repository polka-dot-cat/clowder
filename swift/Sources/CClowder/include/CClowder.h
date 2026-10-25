#ifndef CClowder_h
#define CClowder_h
int clowder_install_signals(void);
int clowder_cancelled(void);
int clowder_run(const char *program, char *const argv[], char *const envp[],
                const char *directory, int input, int output, int error,
                int interactive, int *exit_status);
#endif
