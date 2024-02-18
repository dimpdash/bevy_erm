# Market place

A simple example showing requests generated and processed by bevy. 

9 purchase events are generated which subsequently update the in memory sqlite database. 

Expected output:
```
Creating WebServer with 9 requests to send
Creating tables
        Handling purchase event:
                buyer 1, item 3, request 0v1
        Handling purchase event:
                buyer 1, item 3, request 1v1
        Handling purchase event:
                buyer 1, item 3, request 2v1

Processing purchase events
        Bob The Buyer purchases corn from Alice The Seller
        Bob The Buyer purchases corn from Alice The Seller
        Bob The Buyer purchases corn from Alice The Seller
Finished processing purchase events
Responding to purchase event
Responding to purchase event
Responding to purchase event
        Handling purchase event:
                buyer 1, item 3, request 2v4
        Handling purchase event:
                buyer 1, item 3, request 1v4
        Handling purchase event:
                buyer 1, item 3, request 0v4

Processing purchase events
        Bob The Buyer purchases corn from Alice The Seller
        Bob The Buyer purchases corn from Alice The Seller
        Bob The Buyer purchases corn from Alice The Seller
Finished processing purchase events
Responding to purchase event
Responding to purchase event
Responding to purchase event
        Handling purchase event:
                buyer 1, item 3, request 0v7
        Handling purchase event:
                buyer 1, item 3, request 1v7
        Handling purchase event:
                buyer 1, item 3, request 2v7

Processing purchase events
        Bob The Buyer purchases corn from Alice The Seller
        Bob The Buyer purchases corn from Alice The Seller
        Bob The Buyer purchases corn from Alice The Seller
Finished processing purchase events
Responding to purchase event
Responding to purchase event
Responding to purchase event

============ Exiting ==============
Tables after handling requests

Users
+----+------------------+-------+--------+
| id | name             | buyer | seller |
+----+------------------+-------+--------+
| 1  | Bob The Buyer    | true  | false  |
+----+------------------+-------+--------+
| 2  | Alice The Seller | false | true   |
+----+------------------+-------+--------+
Items
+----+-----------+------+-------+
| id | seller_id | name | price |
+----+-----------+------+-------+
| 3  | 2         | corn | 100   |
+----+-----------+------+-------+
Purchased Items
+----+------+-------+
| id | item | buyer |
+----+------+-------+
| -9 | 3    | 1     |
+----+------+-------+
| -8 | 3    | 1     |
+----+------+-------+
| -7 | 3    | 1     |
+----+------+-------+
| -6 | 3    | 1     |
+----+------+-------+
| -5 | 3    | 1     |
+----+------+-------+
| -4 | 3    | 1     |
+----+------+-------+
| -3 | 3    | 1     |
+----+------+-------+
| -2 | 3    | 1     |
+----+------+-------+
| -1 | 3    | 1     |
+----+------+-------+
```