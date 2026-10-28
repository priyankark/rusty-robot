"""Regression checks: failed delivery must never become a speedup."""
import unittest
from run import summarize

class SummaryTests(unittest.TestCase):
    def row(self, **changes):
        return {'category':'input','backend':'example','profile':'default','case':'text',
                'round':1,'warmup':False,'success':True,'api_ms':10,'observed_ms':20,**changes}

    def test_excludes_failed_delivery_and_warmup_from_latency(self):
        result = summarize([self.row(success=False,api_ms=.001,observed_ms=.001),
                            self.row(warmup=True,api_ms=.002,observed_ms=.002),self.row()])[0]
        self.assertEqual((result['successes'],result['attempts']),(1,2))
        self.assertEqual(result['observed_ms']['median'],20)
        self.assertEqual(result['api_ms']['median'],10)

    def test_no_latency_report_for_all_failed_case(self):
        result=summarize([self.row(success=False)])[0]
        self.assertEqual(result['successes'],0)
        self.assertNotIn('observed_ms',result)
        self.assertNotIn('api_ms',result)

    def test_keeps_rounds_profiles_and_tail_latency(self):
        result=summarize([self.row(observed_ms=10),self.row(observed_ms=30),
                          self.row(round=2,observed_ms=100),self.row(profile='zero')])
        self.assertEqual(len(result),2)
        self.assertEqual(result[0]['observed_ms']['round_medians'],[20,100])
        self.assertEqual(result[0]['observed_ms']['median'],30)
        self.assertEqual(result[0]['observed_ms']['p95'],100)

if __name__=='__main__': unittest.main()
