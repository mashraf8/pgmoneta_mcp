## Copyright (C) 2026 The pgmoneta community
##
## This program is free software: you can redistribute it and/or modify
## it under the terms of the GNU General Public License as published by
## the Free Software Foundation, either version 3 of the License, or
## (at your option) any later version.
##
## This program is distributed in the hope that it will be useful,
## but WITHOUT ANY WARRANTY; without even the implied warranty of
## MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
## GNU General Public License for more details.
##
## You should have received a copy of the GNU General Public License
## along with this program. If not, see <https://www.gnu.org/licenses/>.
FROM docker.io/rockylinux/rockylinux:10

LABEL maintainer="Jesper Pedersen <jesperpedersen.db@gmail.com>"

LABEL summary="PostgreSQL and pgmoneta Composed Image" \
      description="PostgreSQL and pgmoneta Composed Image"

ENV PGVERSION="18"
ENV PGROOT="/usr/pgsql-${PGVERSION}"


RUN mkdir -p /conf-pgmoneta /conf-postgres /pgdata /pgwal /pgmoneta /pglog

# copy config
COPY conf/pgmoneta.conf /conf-pgmoneta/pgmoneta.conf
COPY conf/postgresql.conf /conf-postgres/postgresql.conf
COPY conf/pg_hba.conf /conf-postgres/pg_hba.conf
COPY conf/setup.sql /conf-postgres/setup.sql

# copy executables
COPY usr/bin/run-pgmoneta /usr/bin/run-pgmoneta
COPY usr/bin/run-postgresql /usr/bin/run-postgresql
COPY usr/bin/entry /usr/bin/entry

RUN chmod 755 /usr/bin/run-pgmoneta /usr/bin/run-postgresql /usr/bin/entry

# install needed packages and build dependencies
RUN if ! curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/epel-release.rpm "https://dl.fedoraproject.org/pub/epel/epel-release-latest-10.noarch.rpm"; then \
        if ! curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/epel-release.rpm "https://download.fedoraproject.org/pub/epel/epel-release-latest-10.noarch.rpm"; then \
            curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/epel-release.rpm "https://mirror.netone.nl/other/epel/epel-release-latest-10.noarch.rpm"; \
        fi; \
    fi \
 && rpm -Uvh /tmp/epel-release.rpm \
 && if ! curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/pgdg.rpm "https://download.postgresql.org/pub/repos/yum/reporpms/EL-10-$(uname -m)/pgdg-redhat-repo-latest.noarch.rpm"; then \
        curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/pgdg.rpm "https://ftp.postgresql.org/pub/repos/yum/reporpms/EL-10-$(uname -m)/pgdg-redhat-repo-latest.noarch.rpm"; \
    fi \
 && rpm -Uvh /tmp/pgdg.rpm \
 && rm -f /tmp/epel-release.rpm /tmp/pgdg.rpm \
 && dnf install -y 'dnf-command(config-manager)' \
 && dnf config-manager --set-enabled crb \
 && dnf -y update \
 && dnf -y install wget tar crontabs git gcc make cmake openssl-devel systemd-devel zlib-devel bzip2-devel libarchive-devel libssh-devel libatomic python3-docutils \
 && (dnf -y install ncurses-devel || dnf -y install libncurses-devel) \
 && dnf -y install postgresql18 postgresql18-server postgresql18-contrib postgresql18-libs \
 && (dnf -y install zstd-devel || dnf -y install libzstd-devel) \
 && (dnf -y install lz4-devel || dnf -y install liblz4-devel) \
 && (dnf -y install cjson-devel || dnf -y install libcjson-devel)

# Build libev from source (matching check.sh)
RUN mkdir -p /tmp/libev-src \
 && if ! curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/libev-src/libev.tar.gz "https://dist.schmorp.de/libev/libev-4.33.tar.gz"; then \
        if ! curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/libev-src/libev.tar.gz "https://github.com/enki/libev/archive/refs/tags/v4.33.tar.gz"; then \
            curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/libev-src/libev.tar.gz "https://distfiles.macports.org/libev/libev-4.33.tar.gz"; \
        fi; \
    fi \
 && tar -xzf /tmp/libev-src/libev.tar.gz -C /tmp/libev-src \
 && LIBEV_SRC_DIR="$(find /tmp/libev-src -mindepth 1 -maxdepth 1 -type d | head -n 1)" \
 && cd "$LIBEV_SRC_DIR" \
 && ./configure --prefix=/usr \
 && make -j$(nproc) \
 && make install \
 && rm -rf /tmp/libev-src

# Build libyaml from source (matching check.sh)
RUN dnf install -y autoconf automake libtool \
 && mkdir -p /tmp/libyaml-src \
 && if ! curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/libyaml-src/libyaml.tar.gz "https://github.com/yaml/libyaml/archive/refs/tags/0.2.5.tar.gz"; then \
        curl --retry 3 --retry-all-errors --connect-timeout 2 -fsSL -o /tmp/libyaml-src/libyaml.tar.gz "https://pyyaml.org/download/libyaml/yaml-0.2.5.tar.gz"; \
    fi \
 && tar -xzf /tmp/libyaml-src/libyaml.tar.gz -C /tmp/libyaml-src \
 && LIBYAML_SRC_DIR="$(find /tmp/libyaml-src -mindepth 1 -maxdepth 1 -type d | head -n 1)" \
 && cd "$LIBYAML_SRC_DIR" \
 && if [ -f ./bootstrap ]; then ./bootstrap; fi \
 && ./configure --prefix=/usr \
 && make -j$(nproc) \
 && make install \
 && rm -rf /tmp/libyaml-src

# Build pgmoneta from main branch
RUN git clone --depth 1 --branch main https://github.com/pgmoneta/pgmoneta.git /tmp/pgmoneta-main \
 && mkdir -p /tmp/pgmoneta-main/build \
 && cd /tmp/pgmoneta-main/build \
 && cmake -DDOCS=false -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=/usr .. \
 && make -j$(nproc) \
 && make install \
 && rm -rf /tmp/pgmoneta-main \
 && dnf -y clean all

RUN useradd -ms /bin/bash pgmoneta \
 && mkdir -p /home/pgmoneta/.pgmoneta \
 && chmod 700 /home/pgmoneta/.pgmoneta \
 && chown pgmoneta:pgmoneta /home/pgmoneta/.pgmoneta

## copy master key
COPY --chown=pgmoneta:pgmoneta ./master.key /home/pgmoneta/.pgmoneta/master.key
RUN chmod 600 /home/pgmoneta/.pgmoneta/master.key

RUN chown -R pgmoneta:pgmoneta /conf-pgmoneta /pgmoneta \
 && chown -R postgres:postgres /conf-postgres /pgdata /pgwal /pglog \
 && chmod 700 /conf-pgmoneta /pgmoneta /conf-postgres /pgdata /pgwal /pglog

VOLUME ["/pgdata","/pgwal","/pgmoneta"]

EXPOSE 5001 9100 5432 5002

ENTRYPOINT ["/usr/bin/entry"]
