# Go away! Not done yet... 

## Testing
Run separate validator
``` bash
solana-test-validator --bind-address 0.0.0.0 --url https://api.mainnet-beta.solana.com --ledger .anchor/test-ledger --rpc-port 8899 --clone 7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE --reset
```
then
```bash
anchor test --skip-local-validator
```

or just 
``` bash
anchor test
```