import { AfterViewInit, Component } from '@angular/core';
import { IntersectionService } from '../../services/IntersectionObserver.service';

@Component({
  selector: 'app-summary-bar',
  standalone: true,
  imports: [],
  templateUrl: './summary-bar.component.html',
  styleUrl: './summary-bar.component.css'
})
export class SummaryBarComponent implements AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private intersectionService: IntersectionService
  ) { }

  //  ▬▬▬ Intersection Section <section>
  idActive: string = '';

  // ████ AfterViewInit

  ngAfterViewInit() {
    // Intersection Observer function
    this.intersectionService.getCurrentId().subscribe(id => {
      this.idActive = id;
    })
  }
}
