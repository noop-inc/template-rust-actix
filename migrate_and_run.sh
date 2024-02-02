#!/bin/bash
diesel setup --database-url ${PG__URL}
/app/target/release/actix-api