FROM ubuntu:24.04

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

ENV CESS_GATEWAY="https://deoss-sgp.cess.network"
ENV CESS_TERRITORY="SOMETHING"
ENV PUB_KEY="5Cepa34KmHKM7vbq8P8bz5asVFRYtN1KHk3VKpmkvJindNxy"
ENV MNEMONIC="protect reduce slab market blame foster spread toddler lobster escape forest ozone"
ENV CYBORG_TEST_KEY="bottom drive obey lake curtain smoke basket hold race lonely fit walk//Alice"
ENV RPC_URL="wss://testnet-rpc.cess.network/ws/"

COPY gatekeeper .

RUN chmod +x ./gatekeeper

ENTRYPOINT ["./gatekeeper"]
