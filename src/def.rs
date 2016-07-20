#[macro_use]
use mysql;

use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::vec::Vec;
use std::str;
use std::io::prelude::*;

use parser;
use parser::*;
use nom::IResult;

#[derive(Debug, PartialEq, Eq)]
pub struct DbColumn {
    name: String,
    db_type: String
}

#[derive(Debug, PartialEq, Eq)]
pub struct DbTable {
    pub name: String,
    pub columns: Vec<DbColumn>
}

pub struct GraphQLPool {
    pub pool: mysql::Pool,
    pub database: Vec<DbTable>,
    pub working_database_name: String
}

impl GraphQLPool {
    //db_params has the structure 'username:password@host:port'
    /*
        EXAMPLE:
        db_params = mysql://root:root@localhost:3306,
        "path_to_my_directory/file"
    */
    pub fn new (db_conn: &str, db_name: &str, path_name: &str) -> GraphQLPool{
        let path = Path::new(path_name);
        let mut file = match File::open(path){
            Err(why) => panic!("couldn't open {}: {}", path_name,
                why.description()),
            Ok(file) => file,
        };

        let mut db_data = String::new();
        file.read_to_string(&mut db_data);

        let db = create_database(db_data);

        let mut query: String = "".to_string();
        for table in & db {
            //creates temporary table with auto-generated id
            query = query + "CREATE TEMPORARY TABLE " + db_name + "." + &table.name; query = query + "(
                         " + &table.name + "_id int not null"; for column in &table.columns {query = query + ",
                         "+ &column.name + " "+ &column.db_type}; query = query +"
                     );\n";
        }
        println!("{}", query);

        let p = mysql::Pool::new(db_conn).unwrap();

        let mut conn = p.get_conn().unwrap();
        //conn.query("DROP DATABASE ".to_string() + db_name).unwrap();
        //conn.query("CREATE DATABASE ".to_string() + db_name).unwrap();
        conn.query("USE ".to_string() + db_name).unwrap();

        conn.query(query).unwrap();

        GraphQLPool{
            pool: p,
            database: db,
            working_database_name: db_name.to_string()
        }
    }


    pub fn post (&mut self, query: &str) /*-> Result<T,E>*/ {
        let query_data = sql_insert(query.as_bytes());
        match query_data{
            IResult::Done(input, query_structure) => {
                //query_structure : (&str, Vec<(&str, &str)> )
                let last_column = &query_structure.1.last().unwrap();
                let mut db_query: String = "INSERT INTO ".to_string() + &(self.working_database_name) + "." + query_structure.0 + "(";
                                        /*COLUMNS*/
                                        for col in &query_structure.1{
                                            db_query = db_query + col.0;
                                            if col.0 != last_column.0 {db_query = db_query + ","};
                                            db_query = db_query + " ";
                                        }

                                        db_query = db_query + ")\n" +

                                        "VALUES (";
                                        for col in &query_structure.1{
                                            db_query = db_query + "\"" + col.1 + "\"";;
                                            if col.1 != last_column.1 {db_query = db_query + ","};
                                            db_query = db_query + " ";
                                        }
                                        db_query = db_query + ");";
                println!("Graph_QL_Pool::post:\n{}", db_query);
                self.pool.prep_exec(&db_query, ()).unwrap();
            },
            IResult::Error (cause) => unimplemented!(),
            IResult::Incomplete (size) => unimplemented!()
        }
    }



    pub fn get (&self, query: &str) -> Vec<String> {
        let query_data = sql_select(query.as_bytes());
        match query_data{

            IResult::Done(input, query_structure) => {
                //query_structure : (&str, (&str, &str), Vec<&str>)
                let last_column = query_structure.2.last().unwrap();
                let mut db_query: String = "SELECT ".to_string();
                                            for col in &query_structure.2{
                                                db_query = db_query + col;
                                                if col != last_column {db_query = db_query + ","};
                                                db_query = db_query + " "
                                            }
                                            db_query = db_query + " " +

                                            "FROM " + &(self.working_database_name) + "." + query_structure.0 + " " +

                                            "WHERE " + (query_structure.1).0 + " = " + (query_structure.1).1 + ";";

                println!("Graph_QL_Pool::get:\n{}", db_query);

                let selected_data: Vec<String> = self.pool.prep_exec(db_query, ())
                    .map(|result| {
                        result.map(|x| x.unwrap()).map(|row| {
                            mysql::from_row(row)
                        }).collect() // Collect payments so now `QueryResult` is mapped to `Vec<String>`
                    }).unwrap();
                selected_data
            },
            IResult::Error (cause) => unimplemented!(),
            IResult::Incomplete (size) => unimplemented!()
        }

    }

/*
    pub fn update (&mut self, query: &str) -> Result<T,E> {
        let query_data = sql_update(query);
        match query_data{
            IResult::Done(input, query_structure) => {
                //query_structure = {(&b"user"[..], ("id", "1"), &b"name"[..])}
                let mut query: String = UPDATE t1 SET col1 = col1 + 1;;
                p.prep_exec(&query, ()).unwrap();
            },
            IResult::Error (cause) => unimplemented!(),
            IResult::Incomplete (size) => unimplemented!()
        }
    }

    pub fn delete (&mut self, query: &str) -> Result<T,E> {
        let query_data = sql_delete(query);
        match query_data{
            IResult::Done(input, query_structure) => {
                //query_structure = {(&b"user"[..], ("id", "1"), &b"name"[..])}
                let mut query: String = DELETE t1 FROM test AS t1, test2 WHERE ...;
                p.prep_exec(&query, ()).unwrap();
            },
            IResult::Error (cause) => unimplemented!(),
            IResult::Incomplete (size) => unimplemented!()
        }
    }
*/
}

fn create_database (tables_in_string: String) -> Vec<DbTable> {
    //variables do not live enough, will look at it again later

    let result = parser::database(tables_in_string.as_bytes());

    match result{
        IResult::Done(input, tables) => {
            let mut db: Vec<DbTable> = Vec::new();
            for table in tables {
                let mut columns: Vec<DbColumn> = Vec::new();
                for column in table.1 {
                    columns.push(DbColumn { name: column.0.to_string(), db_type: column.1.to_string() });
                }
                db.push(DbTable{ name: table.0.to_string(), columns:columns })
            }
            db
        },
        IResult::Error (cause) => unimplemented!(),
        IResult::Incomplete (size) => unimplemented!()
    }

    /*sample result:
    IResult::Done(
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
    );*/
    /*
    let result = vec![
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
    ];

    let tables = result;
    let mut db: Vec<DbTable> = Vec::new();
    for table in tables {
        let mut columns: Vec<DbColumn> = Vec::new();
        for column in table.1 {
            columns.push(DbColumn { name: column.0.to_string(), db_type: column.1.to_string() });
        }
        db.push(DbTable{ name: table.0.to_string(), columns:columns })
    }

    db*/
}

// TESTING AREA