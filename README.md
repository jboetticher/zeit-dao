# Zeitgeist DAO

## Potential Improvements
- Tokenized distribution of ZTG from markets
- Additional runtime calls

## Commands

Start local node:  

```bash
./substrate-contracts-node --log info,runtime::contracts=debug 2>&1
```

Instantiate:

```bash
zeit-dao % cargo contract instantiate \
--constructor new \
--args \[5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY] \
--suri //Alice \
-x
```