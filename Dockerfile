FROM rustlang/rust:nightly

WORKDIR /usr/src/myapp
COPY . .

RUN set -ex; \
    apt-get update; \
    apt-get install -q -y --no-install-recommends \
	    gcc-arm-none-eabi \
        ; \
    apt-get autoremove -q -y; \
    apt-get clean -q -y; \
    rm -rf /var/lib/apt/lists/*; \
    cd ..; \
    rustup default nightly-2018-06-12; \
    rustup target add thumbv7em-none-eabihf; \
    cd myapp;
    
