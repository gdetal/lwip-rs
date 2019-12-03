#ifndef LWIP_CUSTOM_LWIPOPTS_H
#define LWIP_CUSTOM_LWIPOPTS_H

#define LWIP_ERRNO_INCLUDE <errno.h>
#define LWIP_ERR_TO_ERRNO 1
#define LWIP_ERR_T err_enum_t

#define LWIP_RAW 1
#define LWIP_UDP 1
#define LWIP_TCP 1
#define LWIP_ARP 0
#define LWIP_ICMP 0
#define LWIP_HAVE_LOOPIF 0
#define LWIP_NETCONN 1

#define LWIP_DONT_PROVIDE_BYTEORDER_FUNCTIONS 1

#define IPV6_FRAG_COPYHEADER 1

#define MEM_LIBC_MALLOC 1
#define MEMP_MEM_MALLOC 1

// Define the netif struct
#define LWIP_IPV4 1
#define LWIP_IPV6 1
#define LWIP_IPV6_NUM_ADDRESSES 3
#define LWIP_NETIF_STATUS_CALLBACK 0
#define LWIP_NETIF_LINK_CALLBACK 0
#define LWIP_DHCP 0
#define LWIP_AUTOIP 0
#define LWIP_IGMP 0
#define LWIP_IPV6_MLD 0
#define LWIP_NUM_NETIF_CLIENT_DATA 0
#define LWIP_IPV6_AUTOCONFIG 1
#define LWIP_IPV6_SEND_ROUTER_SOLICIT 0
#define LWIP_NETIF_HOSTNAME 0
#define LWIP_CHECKSUM_CTRL_PER_NETIF 0
#define MIB2_STATS 0
#define LWIP_NETIF_HWADDRHINT 0
#define LWIP_NETIF_LOOPBACK 0
#define LWIP_NETIF_API 1
#define LWIP_NETIF_REMOVE_CALLBACK 1

// Define the tcp_pcb struct
#define LWIP_TCP_TIMESTAMPS 1

#define IP_TRANSPARENT 1
#define TCP_TRANSPARENT 1

#if FEATURE_DEBUG == 1
#define LWIP_DBG_MIN_LEVEL         LWIP_DBG_LEVEL_ALL
#define LWIP_DEBUG 1
#define RAW_DEBUG                  LWIP_DBG_ON
#define PPP_DEBUG                  LWIP_DBG_ON
#define MEM_DEBUG                  LWIP_DBG_ON
#define MEMP_DEBUG                 LWIP_DBG_ON
#define PBUF_DEBUG                 LWIP_DBG_ON
#define API_LIB_DEBUG              LWIP_DBG_ON
#define API_MSG_DEBUG              LWIP_DBG_ON
#define TCPIP_DEBUG                LWIP_DBG_ON
#define NETIF_DEBUG                LWIP_DBG_ON
#define SOCKETS_DEBUG              LWIP_DBG_ON
#define DNS_DEBUG                  LWIP_DBG_ON
#define AUTOIP_DEBUG               LWIP_DBG_ON
#define DHCP_DEBUG                 LWIP_DBG_ON
#define IP_DEBUG                   LWIP_DBG_ON
#define IP_REASS_DEBUG             LWIP_DBG_ON
#define IP6_DEBUG                  LWIP_DBG_ON
#define ICMP_DEBUG                 LWIP_DBG_ON
#define IGMP_DEBUG                 LWIP_DBG_ON
#define UDP_DEBUG                  LWIP_DBG_ON
#define TCP_DEBUG                  LWIP_DBG_ON
#define TCP_INPUT_DEBUG            LWIP_DBG_ON
#define TCP_OUTPUT_DEBUG           LWIP_DBG_ON
#define TCP_RST_DEBUG              LWIP_DBG_ON
#define TCP_RTO_DEBUG              LWIP_DBG_ON
#define TCP_CWND_DEBUG             LWIP_DBG_ON
#endif

#endif /* LWIP_CUSTOM_LWIPOPTS_H */
