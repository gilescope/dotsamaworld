'use strict';
// import * as smoldot from '@substrate/smoldot-light';
// var smoldot = require('@substrate/smoldot-light');

let current;

const client = smoldot.start();
// client.addChain({
//     chainSpec,
//     jsonRpcCallback: (jsonRpcResponse) => {
//         current = jsonRpcResponse;
//         // Called whenever the client emits a response to a JSON-RPC request,
//         // or an incoming JSON-RPC notification.
//         console.log(jsonRpcResponse)
//     }
// }).then((chain) => {
//     chain.sendJsonRpc('{"jsonrpc":"2.0","id":1,"method":"system_name","params":[]}');
// });

    //     var pollme = function() {
    //     // // Load a string chain specification.
    //     // const chainSpec = fs.readFileSync('./westend.json', 'utf8');

    //     // A single client can be used to initialize multiple chains.
  
    //     return current;
    // }

// module.exports = function () {
//     // // Load a string chain specification.
//     // const chainSpec = fs.readFileSync('./westend.json', 'utf8');

//     // A single client can be used to initialize multiple chains.

//     return current;
// };

export class ISmoldot {
    constructor(id) {
        this.ownable_id = id;
    }

    setup_chain(chainSpec) {
        console.log("setup_chain called");
        console.log(chainSpec);
        client.addChain({
            chainSpec,
            jsonRpcCallback: (jsonRpcResponse) => {
                current = jsonRpcResponse;
                // Called whenever the client emits a response to a JSON-RPC request,
                // or an incoming JSON-RPC notification.
                console.log("Got result back from smoldot:");
                console.log(jsonRpcResponse)
            }
        }).then((chain) => {
            console.log("chain setup. calling rpc...");
            chain.sendJsonRpc('{"jsonrpc":"2.0","id":1,"method":"system_name","params":[]}');
            console.log("rpc called...");
        });
        return "Hello world";
        // return new Promise(async (resolve, reject) => {
        //     setTimeout(() => {
        //         resolve("foo");
        //     }, 300);
        //     // let db = await this.get_db();
        //     // let tx = db.transaction(this.STATE_STORE, this.DB_OP.R)
        //     //     .objectStore(this.STATE_STORE)
        //     //     .get(key);

        //     // tx.onsuccess = () => resolve(tx.result);
        //     // tx.onerror = (e) => reject(e);
        // });
    }

    // async
    pollme() {
        // client.addChain({
        //     chainSpec,
        //     jsonRpcCallback: (jsonRpcResponse) => {
        //         current = jsonRpcResponse;
        //         // Called whenever the client emits a response to a JSON-RPC request,
        //         // or an incoming JSON-RPC notification.
        //         console.log(jsonRpcResponse)
        //     }
        // }).then((chain) => {
        //     chain.sendJsonRpc('{"jsonrpc":"2.0","id":1,"method":"system_name","params":[]}');
        // });
        return "Hello world";
        // return new Promise(async (resolve, reject) => {
        //     setTimeout(() => {
        //         resolve("foo");
        //     }, 300);
        //     // let db = await this.get_db();
        //     // let tx = db.transaction(this.STATE_STORE, this.DB_OP.R)
        //     //     .objectStore(this.STATE_STORE)
        //     //     .get(key);

        //     // tx.onsuccess = () => resolve(tx.result);
        //     // tx.onerror = (e) => reject(e);
        // });
    }
}
