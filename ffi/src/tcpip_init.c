#include "lwip/sys.h"
#include "lwip/tcpip.h"

static void
tcpip_init_priv(void *arg)
{
    sys_sem_t *init_sem = (sys_sem_t *)arg;
    sys_sem_signal(init_sem);
}

err_t
tcpip_init_block(void)
{
    err_t err;
    sys_sem_t init_sem;

    err = sys_sem_new(&init_sem, 0);
    if (err != ERR_OK)
    {
        return err;
    }

    tcpip_init(tcpip_init_priv, &init_sem);

    sys_sem_wait(&init_sem);
    sys_sem_free(&init_sem);

    return ERR_OK;
}