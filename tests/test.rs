#[macro_use]
extern crate rust_sql;

#[macro_use]
extern crate mysql;
extern crate mio;
extern crate eventual;
#[macro_use]
extern crate nom;

#[macro_use]
extern crate log;
extern crate env_logger;


use rust_sql::def::*;
use rust_sql::parser::*;

//use mio::{Token, EventLoop};

use eventual::*;

use mysql as my;

//use std::fs::File;
//use std::path::Path;
use std::io::prelude::*;
use std::str;
use std::str::FromStr;
use std::vec::Vec;

use nom::{IResult,digit};
use nom::IResult::*;

const DB_NAME: &'static str = "db";
const DB_USER: &'static str = "root";
const DB_PASSWORD: &'static str = "root";
const HOST: &'static str = "localhost";
const PORT: &'static str = "3306";



#[test]
fn test_mysql_module(){
    #[derive(Debug, PartialEq, Eq)]
    struct Payment {
        customer_id: i32,
        amount: i32,
        account_name: Option<String>,
    }

    let MYSQL_CONNECTION = "mysql://".to_string()+DB_USER+":"+DB_PASSWORD+"@"+HOST+":"+PORT;
    let pool = my::Pool::new(MYSQL_CONNECTION.as_str()).unwrap(); //mysql://username:password@host:port
    let mut conn = pool.get_conn().unwrap();
    conn.query("DROP DATABASE ".to_string() + DB_NAME).unwrap();
    conn.query("CREATE DATABASE ".to_string() + DB_NAME).unwrap();

    // Let's create payment table.
    // It is temporary so we do not need `tmp` database to exist.
    // Unwap just to make sure no error happened.
    pool.prep_exec("CREATE TEMPORARY TABLE ".to_string() + DB_NAME + ". payment (
                         customer_id int not null,
                         amount int not null,
                         account_name text
                     )", ()).unwrap();

    let payments = vec![
        Payment { customer_id: 1, amount: 2, account_name: None },
        Payment { customer_id: 3, amount: 4, account_name: Some("foo".into()) },
        Payment { customer_id: 5, amount: 6, account_name: None },
        Payment { customer_id: 7, amount: 8, account_name: None },
        Payment { customer_id: 9, amount: 10, account_name: Some("bar".into()) },
    ];

    // Let's insert payments to the database
    // We will use into_iter() because we do not need to map Stmt to anything else.
    // Also we assume that no error happened in `prepare`.
    for mut stmt in pool.prepare(r"INSERT INTO  ".to_string() + DB_NAME + ". payment
                                       (customer_id, amount, account_name)
                                   VALUES
                                       (:customer_id, :amount, :account_name)").into_iter() {
        for p in payments.iter() {
            // `execute` takes ownership of `params` so we pass account name by reference.
            // Unwrap each result just to make sure no errors happened.
            stmt.execute(params!{
                "customer_id" => my::Value::from(p.customer_id),
                "amount" => my::Value::from(p.amount),
                "account_name" => my::Value::from(&p.account_name),
            }).unwrap();
        }
    }

    // Let's select payments from database
    let selected_payments: Vec<Payment> =
    pool.prep_exec("SELECT customer_id, amount, account_name from ".to_string() + DB_NAME + ".payment", ())
        .map(|result| { // In this closure we sill map `QueryResult` to `Vec<Payment>`
            // `QueryResult` is iterator over `MyResult<row, err>` so first call to `map`
            // will map each `MyResult` to contained `row` (no proper error handling)
            // and second call to `map` will map each `row` to `Payment`
            result.map(|x| x.unwrap()).map(|row| {
                let (customer_id, amount, account_name) = my::from_row(row);
                Payment {
                    customer_id: customer_id,
                    amount: amount,
                    account_name: account_name,
                }
            }).collect() // Collect payments so now `QueryResult` is mapped to `Vec<Payment>`
        }).unwrap(); // Unwrap `Vec<Payment>`

    // Now make sure that `payments` equals to `selected_payments`.
    // Mysql gives no guaranties on order of returned rows without `ORDER BY`
    // so assume we are lukky.
    assert_eq!(payments, selected_payments);

}

#[test]
fn test_nom_module(){
    named!(parens<i64>, delimited!(
        char!('('),
        expr,
        char!(')')
      )
    );

    named!(i64_digit<i64>,
      map_res!(
        map_res!(
          digit,
          str::from_utf8
        ),
        FromStr::from_str
      )
    );

    // We transform an integer string into a i64
    // we look for a digit suite, and try to convert it.
    // if either str::from_utf8 or FromStr::from_str fail,
    // the parser will fail
    named!(factor<i64>,
      alt!(
        i64_digit
      | parens
      )
    );

    // we define acc as mutable to update its value whenever a new term is found
    named!(term <i64>,
      chain!(
        mut acc: factor  ~
                 many0!(
                   alt!(
                     tap!(mul: preceded!(tag!("*"), factor) => acc = acc * mul) |
                     tap!(div: preceded!(tag!("/"), factor) => acc = acc / div)
                   )
                 ),
        || { return acc }
      )
    );

    named!(expr <i64>,
      chain!(
        mut acc: term  ~
                 many0!(
                   alt!(
                     tap!(add: preceded!(tag!("+"), term) => acc = acc + add) |

                     tap!(sub: preceded!(tag!("-"), term) => acc = acc - sub)
                   )
                 ),
        || { return acc }
      )
    );

    assert_eq!(expr(b"1+2"),         IResult::Done(&b""[..], 3));
    assert_eq!(expr(b"12+6-4+3"),    IResult::Done(&b""[..], 17));
    assert_eq!(expr(b"1+2*3+4"),     IResult::Done(&b""[..], 11));

    assert_eq!(expr(b"(2)"),         IResult::Done(&b""[..], 2));
    assert_eq!(expr(b"2*(3+4)"),     IResult::Done(&b""[..], 14));
    assert_eq!(expr(b"2*2/(5-1)+3"), IResult::Done(&b""[..], 4));


    named!(alt_tags, alt!(tag!("abcd") | tag!("efgh")));
    assert_eq!(alt_tags(b"abcdxxx"), Done(b"xxx" as &[u8], b"abcd" as &[u8]));
    assert_eq!(alt_tags(b"efghxxx"), Done(b"xxx" as &[u8], b"efgh" as &[u8]));
    //assert_eq!(alt_tags(b"ijklxxx"), Error(1));

    named!( not_space, is_not!( " \t\r\n" ) );

    let r = not_space(&b"abcdefgh\nijklmnopqrstuvwxyz"[..]);
    assert_eq!(r, Done(&b"\nijklmnopqrstuvwxyz"[..], &b"abcdefgh"[..]));
}


/*
const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

#[test]
fn test_mio_module () {
        debug!("Starting TEST_ECHO_SERVER");
        let mut event_loop = EventLoop::new().unwrap();

        let addr = localhost();
        let srv = TcpListener::bind(&addr).unwrap();

        info!("listen for connections");
        event_loop.register(&srv, SERVER, EventSet::readable(),
                            PollOpt::edge() | PollOpt::oneshot()).unwrap();

        let sock = TcpStream::connect(&addr).unwrap();

        // Connect to the server
        event_loop.register(&sock, CLIENT, EventSet::writable(),
                            PollOpt::edge() | PollOpt::oneshot()).unwrap();

        // Start the event loop
        event_loop.run(&mut Echo::new(srv, sock, vec!["foo", "bar"])).unwrap();
}
*/

#[test]
fn test_eventual () {
    // Run a computation in another thread
    let future1 = Future::spawn(|| {
        // Represents an expensive computation, but for now just return a
        // number
        42
    });

    // Run another computation
    let future2 = Future::spawn(|| {
        // Another expensive computation
        18
    });

    let res = join((
                       future1.map(|v| v * 2),
                       future2.map(|v| v + 5)))
        .and_then(|(v1, v2)| Ok(v1 - v2))
        .await().unwrap();

    assert_eq!(61, res);
}

#[test]
//#[ignore] //hi ha un problema al crear la db
fn test_db_creation () {

    #[derive(Debug, PartialEq, Eq)]
    struct User {
        id: i32,
        name: String
    }

    let MYSQL_CONNECTION = "mysql://".to_string()+DB_USER+":"+DB_PASSWORD+"@"+HOST+":"+PORT;
    let graph_ql_pool = GraphQLPool::new(
        MYSQL_CONNECTION.as_str(),
        DB_NAME,
        "/home/serhiy/Desktop/rust-sql/types"
    );

    graph_ql_pool.pool.prep_exec(r"CREATE TEMPORARY TABLE ".to_string() + DB_NAME + ".user (
                         id int not null,
                         name text
                     )", ()).unwrap();

    let users = vec![
        User { id: 1, name: "foo".into() },
        User { id: 2, name: "bar".into() }
    ];

    for mut stmt in graph_ql_pool.pool.prepare(r"INSERT INTO ".to_string() + DB_NAME + ".user
                                       (id, name)
                                   VALUES
                                       (:id, :name)").into_iter() {
        for p in users.iter() {
            stmt.execute(params!{
                "id" => my::Value::from(p.id),
                "name" => my::Value::from(&p.name),
            }).unwrap();
        }
    }

    let selected_users: Vec<User> =
    graph_ql_pool.pool.prep_exec("SELECT * FROM ".to_string() + DB_NAME + ".user", ())
        .map(|result| {
            result.map(|x| x.unwrap()).map(|row| {
                let (id, name) = my::from_row(row);
                User {
                    id: id,
                    name: name,
                }
            }).collect()
        }).unwrap();

    assert_eq!(users, selected_users);

}

/*
//It works if not letting read file in the method GraphQLPool::new
#[test]
fn test_db_table_creation_from_file(){
    let mut graph_ql_pool = GraphQLPool::new(
        "mysql://root:root@localhost:3306",
        "/home/serhiy/Desktop/rust-sql/types"
    );
    let mut s = String::new();
    graph_ql_pool.init_db_file.read_to_string(&mut s);
    println!("{}", s);

    let result_database = IResult::Done(
        &b""[..],
        vec![
            {
                (&"Human"[..],
                vec![
                    {("id", "String")},
                    {("name", "String")},
                    {("homePlanet", "String")}
                ])
            },
            {
                (&"Droid"[..],
                vec![
                    {("id", "String")},
                    {("name", "String")},
                    {("primaryFunction", "String")}
                ])
            }
        ]
    );
    assert_eq!(
        database(&s.into_bytes()[..]),
        result_database
    );
}
*/


    /*


    let expected_answer =
    "
        {
            'data': {
                'user': {
                    'name': 'foo'
                }
            }
        }
    "
    ;
    let answer = graphQLPool.graphql(query);
    */