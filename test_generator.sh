#!/bin/zsh

TYPES=("deposit" "withdrawal" "dispute" "resolve" "chargeback")
TX=1
CLIENTS=(0)
TXS=(0)

echo "type,client,tx,amount" > test.csv
echo "deposit,0,0,$((RANDOM % 1000))" >> test.csv
for i in {1..1000}; do
    operation=${TYPES[$((RANDOM % 5 + 1))]}
    if [ $operation = "deposit" ] || [ $operation = "withdrawal" ]; then
        client=$((RANDOM % 5))
        if [ $operation = "deposit" ]; then
            CLIENTS+=($client)
            TXS+=($TX)
        fi
        echo "$operation,$client,$TX,$((RANDOM % 1000))" >> test.csv
    elif [ $operation = "dispute" ] || [ $operation = "resolve" ] || [ $operation = "chargeback" ]; then
        num=$((RANDOM % $#CLIENTS + 1))
        echo "$operation,${CLIENTS[$num]},${TXS[$num]}" >> test.csv
    fi
    TX=$((TX + 1))
done
