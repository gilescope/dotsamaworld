// import * as smoldot from '@substrate/smoldot-light';
var smoldot = require('@substrate/smoldot-light');

let current;

const client = smoldot.start();
client.addChain({
    chainSpec,
    jsonRpcCallback: (jsonRpcResponse) => {
        current = jsonRpcResponse;
        // Called whenever the client emits a response to a JSON-RPC request,
        // or an incoming JSON-RPC notification.
        console.log(jsonRpcResponse)
    }
}).then((chain) => {
    chain.sendJsonRpc('{"jsonrpc":"2.0","id":1,"method":"system_name","params":[]}');
})

    function poll() {
        // // Load a string chain specification.
        // const chainSpec = fs.readFileSync('./westend.json', 'utf8');

        // A single client can be used to initialize multiple chains.
  
        return current;
    }
module.export = { poll: poll };
